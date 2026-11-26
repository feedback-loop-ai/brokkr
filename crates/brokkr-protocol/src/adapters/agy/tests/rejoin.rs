//! Rejoin state retention and `root_session` tests for `brokkr driver agy`
//! (#1227, decision 0075 level 2 slice 1).

use std::ffi::{OsStr, OsString};
use std::io::ErrorKind;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{symlink, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use super::judged::bound;
use super::keep::{kept_parent, run_seat_agy, OK_RES, STEP_DONE, UUID};
use super::*;
use crate::adapters::agy::keep::{self as agy_keep, KeepNotice, ParentError, SourceErrno};
use crate::adapters::agy::rejoin::{
    for_each_dir_entry, for_each_listed_entry, keep_rejoin, keep_rejoin_with, same_stamp,
    CopyPoint, RejoinCaps, RejoinNotice, RejoinRollback, REJOIN_BRAIN_DIR, REJOIN_DB_FILE,
    REJOIN_DIR,
};
use crate::adapters::spawn::observed_version_in;
use crate::secret::SecretMatcher;
use crate::DeclaredTranscriptKind;

fn write_exec(path: &Path, body: &str) -> PathBuf {
    std::fs::write(path, body).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path.to_path_buf()
}

fn setup_planted_with_transcript(
    db: &[u8],
    files: &[(&str, &[u8])],
    transcript: &[u8],
) -> (tempfile::TempDir, PathBuf, PathBuf, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let (engine, op) = (dir.path().join("engine"), dir.path().join("op"));
    plant_conversation(&engine, UUID, db, files);
    let conv = prep_kept_transcript(&op, UUID, transcript);
    (dir, engine, op, conv)
}

fn conv_dir(op_home: impl AsRef<Path>, id: &str) -> PathBuf {
    kept_parent(op_home.as_ref()).join(id)
}

fn plant_conversation(engine_home: &Path, id: &str, db: &[u8], brain_files: &[(&str, &[u8])]) {
    let convs = engine_home.join(".gemini/antigravity-cli/conversations");
    std::fs::create_dir_all(&convs).unwrap();
    let db_path = convs.join(format!("{id}.db"));
    std::fs::write(&db_path, db).unwrap();
    std::fs::set_permissions(&db_path, std::fs::Permissions::from_mode(0o644)).unwrap();

    let brain = engine_home.join(".gemini/antigravity-cli/brain").join(id);
    std::fs::create_dir_all(&brain).unwrap();
    for (rel, bytes) in brain_files {
        let path = brain.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
            std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        std::fs::write(&path, bytes).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    }
}

fn prep_kept_transcript(op_home: &Path, id: &str, contents: &[u8]) -> PathBuf {
    let conv = conv_dir(op_home, id);
    std::fs::create_dir_all(&conv).unwrap();
    std::fs::set_permissions(
        conv.parent().unwrap(),
        std::fs::Permissions::from_mode(0o700),
    )
    .unwrap();
    std::fs::set_permissions(&conv, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::write(conv.join("transcript.jsonl"), contents).unwrap();
    conv
}

fn init_line(id: &str, model: &str) -> String {
    format!(
        "{{\"event\":\"init\",\"conversation_id\":\"{id}\",\"init\":{{\"model\":\"{model}\"}}}}"
    )
}

fn declared_work_input(world: &World) -> Value {
    let mut input = world.input(None);
    input["resume_context"] = json!({
        "assessment": Value::Null,
        "declared_transcript_kind": "agy-conversation",
        "declared_root_session_kind": "agy-conversation",
    });
    input
}

fn harness_started(messages: &[crate::Body]) -> Option<&Value> {
    messages.iter().find_map(|body| match body {
        crate::Body::Checkpoint { data, .. } if data["step"] == "harness-started" => Some(data),
        _ => None,
    })
}

fn assert_no_root_session(messages: &[crate::Body]) {
    assert!(messages.iter().all(|body| !matches!(
        body,
        crate::Body::Checkpoint { data, .. } if data.get("root_session").is_some()
    )));
}

fn write_rejoin_fake_agy(
    dir: &Path,
    version: (&str, i32),
    id: &str,
    db: Option<&[u8]>,
    step_bytes: Option<&[u8]>,
    stdout: &str,
) -> PathBuf {
    std::fs::write(dir.join("stdout"), stdout).unwrap();
    std::fs::write(dir.join("version.out"), version.0).unwrap();
    let d = dir.display();
    let db_cmd = db.map_or_else(String::new, |bytes| {
        std::fs::write(dir.join("conv.db"), bytes).unwrap();
        format!(
            "mkdir -p \"$HOME/.gemini/antigravity-cli/conversations\"\n\
             cp '{d}/conv.db' \"$HOME/.gemini/antigravity-cli/conversations/{id}.db\"\n\
             chmod 0644 \"$HOME/.gemini/antigravity-cli/conversations/{id}.db\"\n"
        )
    });
    let step_cmd = step_bytes.map_or_else(String::new, |bytes| {
        std::fs::write(dir.join("step.out"), bytes).unwrap();
        format!(
            "mkdir -p \"$HOME/.gemini/antigravity-cli/brain/{id}/.system_generated/steps/1\"\n\
             cp '{d}/step.out' \"$HOME/.gemini/antigravity-cli/brain/{id}/.system_generated/steps/1/output.txt\"\n\
             chmod 0644 \"$HOME/.gemini/antigravity-cli/brain/{id}/.system_generated/steps/1/output.txt\"\n"
        )
    });
    let script = format!(
        "#!/bin/sh\n\
         if [ \"$1\" = \"--version\" ]; then\n\
           printf '%s\\n%s\\n' \"$AGY_CLI_DISABLE_AUTO_UPDATE\" \"$HOME\" > '{d}/version.env'\n\
           cat '{d}/version.out'; exit {code}\n\
         fi\n\
         line=$(cat)\n\
         logs=\"$HOME/.gemini/antigravity-cli/brain/{id}/.system_generated/logs\"\n\
         mkdir -p \"$logs\"\n\
         printf '{{\"step_index\":0,\"role\":\"USER\",\"text\":\"hi\",\"created_at\":\"2026-11-26T00:00:00Z\"}}\\n' > \"$logs/transcript.jsonl\"\n\
         chmod 0644 \"$logs/transcript.jsonl\"\n\
         {db_cmd}{step_cmd}\
         target=$(printf '%s' \"$line\" | sed -n 's/.*\\\\n    \\(\\/[^\\\\\"]*\\.json\\)\\\\n.*/\\1/p' | head -1)\n\
         [ -n \"$target\" ] && printf '{{\"result\":\"complete\"}}' > \"$target\"\n\
         cat '{d}/stdout'; exit 0\n",
        code = version.1
    );
    write_exec(&dir.join("agy"), &script)
}

#[test]
fn secret_matcher_detects_raw_json_escaped_and_split_chunks_and_resets() {
    let mut empty = SecretMatcher::new(&[]);
    assert!(!empty.feed(b"anything at all"));

    let binding = bound("API_TOKEN", r#"tok"en\val-99"#);
    let mut matcher = SecretMatcher::new(&[binding]);

    assert!(!matcher.feed(b"ordinary conversation content"));
    matcher.reset();
    assert!(matcher.feed(b"prefix tok\"en\\val-99 suffix"));

    matcher.reset();
    assert!(!matcher.feed(b"prefix tok\"en"));
    assert!(matcher.feed(b"\\val-99 suffix"));

    matcher.reset();
    assert!(!matcher.feed(b"end-of-file tok\"en"));
    matcher.reset();
    assert!(!matcher.feed(b"\\val-99 start-of-next-file"));

    matcher.reset();
    assert!(matcher.feed(br#"{"out":"tok\"en\\val-99"}"#));
    assert!(matcher.feed(b"trailing bytes after match without reset"));
    assert!(matcher.feed(b""));
    matcher.reset();
    assert!(!matcher.feed(b"clean after reset"));
}

#[test]
fn keep_rejoin_copies_whole_tree_operator_only_and_refuses_duplicate_or_bad_id() {
    let dir = tempfile::tempdir().unwrap();
    let (engine, op) = (dir.path().join("engine"), dir.path().join("op"));
    std::fs::create_dir_all(&op).unwrap();
    plant_conversation(
        &engine,
        UUID,
        b"sqlite-db-bytes",
        &[
            (".system_generated/logs/transcript.jsonl", b"row1\n"),
            (".system_generated/steps/1/output.txt", b"step-output"),
        ],
    );

    for bad in [
        "..",
        "../x",
        "not-a-uuid",
        "32786EB8-d649-4c35-9110-c8c87a1cdb7d",
    ] {
        assert_eq!(
            keep_rejoin(bad, &engine, Some(&op), &[], false, RejoinCaps::DEFAULT),
            Err(RejoinNotice::InvalidId)
        );
    }

    keep_rejoin(UUID, &engine, Some(&op), &[], false, RejoinCaps::DEFAULT).unwrap();
    let conv = conv_dir(&op, UUID);
    let rejoin = conv.join(REJOIN_DIR);
    let db = rejoin.join(REJOIN_DB_FILE);
    let brain = rejoin.join(REJOIN_BRAIN_DIR);
    let step_dir = brain.join(".system_generated/steps/1");
    let step_file = step_dir.join("output.txt");

    for d in [&conv, &rejoin, &brain, &step_dir] {
        assert_eq!(std::fs::metadata(d).unwrap().mode() & 0o777, 0o700);
    }
    for f in [&db, &step_file] {
        assert_eq!(std::fs::metadata(f).unwrap().mode() & 0o777, 0o600);
    }
    assert_eq!(std::fs::read(&db).unwrap(), b"sqlite-db-bytes");
    assert_eq!(std::fs::read(&step_file).unwrap(), b"step-output");
    assert_eq!(
        keep_rejoin(UUID, &engine, Some(&op), &[], true, RejoinCaps::DEFAULT),
        Err(RejoinNotice::AlreadyKept)
    );
    assert_eq!(
        keep_rejoin(UUID, &engine, Some(&op), &[], false, RejoinCaps::DEFAULT),
        Err(RejoinNotice::AlreadyKept)
    );
}

#[test]
fn bound_value_in_db_or_brain_blocks_rejoin_and_spares_transcript() {
    let secret_val = "l2b-sentinel-value-7f3a9c2e41d8";
    let db_bytes = format!("db-header-{secret_val}-tail");
    let (dir, engine, op, conv) = setup_planted_with_transcript(
        db_bytes.as_bytes(),
        &[(".system_generated/logs/transcript.jsonl", b"clean\n")],
        b"kept-transcript",
    );

    assert_eq!(
        keep_rejoin(
            UUID,
            &engine,
            Some(&op),
            &[bound("L2B_SENTINEL", secret_val)],
            true,
            RejoinCaps::DEFAULT,
        ),
        Err(RejoinNotice::BoundValueAppeared)
    );
    assert!(!conv.join(REJOIN_DIR).exists());
    assert_eq!(
        std::fs::read(conv.join("transcript.jsonl")).unwrap(),
        b"kept-transcript"
    );

    // Split across 64 KiB chunk boundary in a brain file -> rolled back.
    let (engine2, op2) = (dir.path().join("engine2"), dir.path().join("op2"));
    std::fs::create_dir_all(&op2).unwrap();
    let mut big = vec![b'x'; 65_530];
    big.extend_from_slice(secret_val.as_bytes());
    big.extend_from_slice(b"tail");
    plant_conversation(&engine2, UUID, b"clean-db", &[("steps/2/output.txt", &big)]);
    assert_eq!(
        keep_rejoin(
            UUID,
            &engine2,
            Some(&op2),
            &[bound("L2B_SENTINEL", secret_val)],
            false,
            RejoinCaps::DEFAULT,
        ),
        Err(RejoinNotice::BoundValueAppeared)
    );
    assert!(!conv_dir(&op2, UUID).exists());

    // Unrelated secret not present -> succeeds.
    keep_rejoin(
        UUID,
        &engine2,
        Some(&op2),
        &[bound("UNRELATED", "absent-secret-987654")],
        false,
        RejoinCaps::DEFAULT,
    )
    .unwrap();
    assert!(conv_dir(&op2, UUID)
        .join(REJOIN_DIR)
        .join(REJOIN_DB_FILE)
        .exists());
}

#[test]
fn bounds_and_missing_parts_roll_back_cleanly() {
    let dir = tempfile::tempdir().unwrap();
    let op = dir.path().join("op");
    std::fs::create_dir_all(&op).unwrap();

    let eng = dir.path().join("eng-cap");
    plant_conversation(&eng, UUID, b"12345", &[("a.txt", b"67890")]);
    assert_eq!(
        keep_rejoin(
            UUID,
            &eng,
            Some(&op),
            &[],
            false,
            RejoinCaps {
                bytes: 9,
                entries: 10,
                depth: 4
            }
        ),
        Err(RejoinNotice::TooLarge(9))
    );
    assert!(!conv_dir(&op, UUID).exists());
    // Budget charges conversation.db (1) + brain root (2) + a.txt (3) = 3 entries.
    for entry_cap in [0, 1, 2] {
        assert_eq!(
            keep_rejoin(
                UUID,
                &eng,
                Some(&op),
                &[],
                false,
                RejoinCaps {
                    bytes: 100,
                    entries: entry_cap,
                    depth: 4
                }
            ),
            Err(RejoinNotice::TooManyEntries(entry_cap)),
            "entry_cap={entry_cap}"
        );
        assert!(!conv_dir(&op, UUID).exists(), "entry_cap={entry_cap}");
    }
    keep_rejoin(
        UUID,
        &eng,
        Some(&op),
        &[],
        false,
        RejoinCaps {
            bytes: 100,
            entries: 3,
            depth: 4,
        },
    )
    .unwrap();
    std::fs::remove_dir_all(conv_dir(&op, UUID)).unwrap();

    let deep = dir.path().join("eng-deep");
    plant_conversation(&deep, UUID, b"db", &[("a/b/c.txt", b"ok")]);
    assert_eq!(
        keep_rejoin(
            UUID,
            &deep,
            Some(&op),
            &[],
            false,
            RejoinCaps {
                bytes: 100,
                entries: 20,
                depth: 1
            }
        ),
        Err(RejoinNotice::TooDeep(1))
    );
    assert!(!conv_dir(&op, UUID).exists());

    let no_db = dir.path().join("no-db");
    std::fs::create_dir_all(no_db.join(".gemini/antigravity-cli/conversations")).unwrap();
    std::fs::create_dir_all(no_db.join(".gemini/antigravity-cli/brain").join(UUID)).unwrap();
    assert_eq!(
        keep_rejoin(UUID, &no_db, Some(&op), &[], false, RejoinCaps::DEFAULT),
        Err(RejoinNotice::Source(SourceErrno::Absent))
    );

    let no_brain = dir.path().join("no-brain");
    std::fs::create_dir_all(no_brain.join(".gemini/antigravity-cli/conversations")).unwrap();
    std::fs::write(
        no_brain
            .join(".gemini/antigravity-cli/conversations")
            .join(format!("{UUID}.db")),
        b"db",
    )
    .unwrap();
    std::fs::create_dir_all(no_brain.join(".gemini/antigravity-cli/brain")).unwrap();
    assert_eq!(
        keep_rejoin(UUID, &no_brain, Some(&op), &[], false, RejoinCaps::DEFAULT),
        Err(RejoinNotice::Source(SourceErrno::Absent))
    );
}

#[test]
fn unsafe_sources_roll_back_rejoin_cleanly() {
    let dir = tempfile::tempdir().unwrap();
    let op = dir.path().join("op");
    let decoy = dir.path().join("decoy.txt");
    std::fs::create_dir_all(&op).unwrap();
    std::fs::write(&decoy, b"secret").unwrap();

    // Directory where db belongs.
    let dir_db = dir.path().join("dir-db");
    std::fs::create_dir_all(
        dir_db
            .join(".gemini/antigravity-cli/conversations")
            .join(format!("{UUID}.db")),
    )
    .unwrap();
    std::fs::create_dir_all(dir_db.join(".gemini/antigravity-cli/brain").join(UUID)).unwrap();
    assert_eq!(
        keep_rejoin(UUID, &dir_db, Some(&op), &[], false, RejoinCaps::DEFAULT),
        Err(RejoinNotice::Source(SourceErrno::Unsafe))
    );

    // Symlinked file and symlinked directory inside brain each roll back
    // already-copied conversation.db without following the link.
    let outside_dir = dir.path().join("outside-dir");
    std::fs::create_dir_all(&outside_dir).unwrap();
    std::fs::write(outside_dir.join("leak.txt"), b"outside").unwrap();
    for (case_dir, link_name, target) in [
        ("link-bf", "evil.txt", &decoy),
        ("link-bsub", "sublink", &outside_dir),
    ] {
        let eng = dir.path().join(case_dir);
        plant_conversation(&eng, UUID, b"db", &[("good.txt", b"ok")]);
        symlink(
            target,
            eng.join(".gemini/antigravity-cli/brain")
                .join(UUID)
                .join(link_name),
        )
        .unwrap();
        assert_eq!(
            keep_rejoin(UUID, &eng, Some(&op), &[], false, RejoinCaps::DEFAULT),
            Err(RejoinNotice::Source(SourceErrno::Unsafe)),
            "{case_dir}"
        );
        assert!(!conv_dir(&op, UUID).exists(), "{case_dir}");
    }

    // Symlink at db and symlink at brain root.
    let link_db = dir.path().join("link-db");
    std::fs::create_dir_all(link_db.join(".gemini/antigravity-cli/conversations")).unwrap();
    symlink(
        &decoy,
        link_db
            .join(".gemini/antigravity-cli/conversations")
            .join(format!("{UUID}.db")),
    )
    .unwrap();
    std::fs::create_dir_all(link_db.join(".gemini/antigravity-cli/brain").join(UUID)).unwrap();
    assert_eq!(
        keep_rejoin(UUID, &link_db, Some(&op), &[], false, RejoinCaps::DEFAULT),
        Err(RejoinNotice::Source(SourceErrno::Unsafe))
    );

    let link_brain = dir.path().join("link-brain");
    std::fs::create_dir_all(link_brain.join(".gemini/antigravity-cli/conversations")).unwrap();
    std::fs::write(
        link_brain
            .join(".gemini/antigravity-cli/conversations")
            .join(format!("{UUID}.db")),
        b"db",
    )
    .unwrap();
    symlink(&op, link_brain.join(".gemini/antigravity-cli/brain")).unwrap();
    assert_eq!(
        keep_rejoin(
            UUID,
            &link_brain,
            Some(&op),
            &[],
            false,
            RejoinCaps::DEFAULT
        ),
        Err(RejoinNotice::Source(SourceErrno::Unsafe))
    );
}

#[test]
fn unreadable_sources_and_dest_io_errors_bind() {
    let dir = tempfile::tempdir().unwrap();
    let op = dir.path().join("op");
    std::fs::create_dir_all(&op).unwrap();
    let unreadable = dir.path().join("unreadable");
    plant_conversation(&unreadable, UUID, b"db", &[("locked.txt", b"ok")]);
    let locked_file = unreadable
        .join(".gemini/antigravity-cli/brain")
        .join(UUID)
        .join("locked.txt");
    std::fs::set_permissions(&locked_file, std::fs::Permissions::from_mode(0o000)).unwrap();
    let f_err = keep_rejoin(
        UUID,
        &unreadable,
        Some(&op),
        &[],
        false,
        RejoinCaps::DEFAULT,
    );
    std::fs::set_permissions(&locked_file, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(
        f_err,
        Err(RejoinNotice::Source(SourceErrno::Io(
            ErrorKind::PermissionDenied
        )))
    );
    assert!(!conv_dir(&op, UUID).exists());

    let locked_dir = unreadable.join(".gemini/antigravity-cli/brain").join(UUID);
    std::fs::set_permissions(&locked_dir, std::fs::Permissions::from_mode(0o000)).unwrap();
    let d_err = keep_rejoin(
        UUID,
        &unreadable,
        Some(&op),
        &[],
        false,
        RejoinCaps::DEFAULT,
    );
    std::fs::set_permissions(&locked_dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(
        d_err,
        Err(RejoinNotice::Source(SourceErrno::Io(
            ErrorKind::PermissionDenied
        )))
    );

    let locked_sub_eng = dir.path().join("unreadable-sub");
    plant_conversation(&locked_sub_eng, UUID, b"db", &[("sub/file.txt", b"ok")]);
    let locked_sub = locked_sub_eng
        .join(".gemini/antigravity-cli/brain")
        .join(UUID)
        .join("sub");
    std::fs::set_permissions(&locked_sub, std::fs::Permissions::from_mode(0o000)).unwrap();
    let sub_err = keep_rejoin(
        UUID,
        &locked_sub_eng,
        Some(&op),
        &[],
        false,
        RejoinCaps::DEFAULT,
    );
    std::fs::set_permissions(&locked_sub, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(
        sub_err,
        Err(RejoinNotice::Source(SourceErrno::Io(
            ErrorKind::PermissionDenied
        )))
    );
    assert!(!conv_dir(&op, UUID).exists());

    // Read-only conversation directory exercises RejoinRollback io_errno.
    let ro_op = dir.path().join("ro-op");
    let ro_conv = prep_kept_transcript(&ro_op, UUID, b"keep");
    std::fs::set_permissions(&ro_conv, std::fs::Permissions::from_mode(0o500)).unwrap();
    let ro_err = keep_rejoin(
        UUID,
        &unreadable,
        Some(&ro_op),
        &[],
        true,
        RejoinCaps::DEFAULT,
    );
    std::fs::set_permissions(&ro_conv, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(ro_err, Err(RejoinNotice::Io(ErrorKind::PermissionDenied)));

    // Write to a read-only destination File exercises stream_copy_and_screen io_err.
    let src_p = dir.path().join("src.bin");
    let dst_p = dir.path().join("dst.bin");
    std::fs::write(&src_p, b"payload").unwrap();
    std::fs::write(&dst_p, b"").unwrap();
    let mut src_f = std::fs::File::open(&src_p).unwrap();
    let mut ro_dst = std::fs::File::open(&dst_p).unwrap();
    assert!(matches!(
        super::super::rejoin::stream_copy_for_test(&mut src_f, &mut ro_dst, 64),
        Err(RejoinNotice::Io(_))
    ));
}

#[test]
fn changed_underfoot_and_streaming_overflow_roll_back_and_spare_transcript() {
    let (_dir, engine, op, conv) =
        setup_planted_with_transcript(b"db12", &[("a.txt", b"hello")], b"keep-me");
    let target = engine
        .join(".gemini/antigravity-cli/brain")
        .join(UUID)
        .join("a.txt");

    let run_mut = |caps: RejoinCaps, mut mutate: Box<dyn FnMut()>| {
        keep_rejoin_with(UUID, &engine, Some(&op), &[], true, caps, &mut |point| {
            if matches!(point, CopyPoint::ReadFile(n) if n == OsStr::new("a.txt")) {
                mutate();
            }
        })
    };

    let t1 = target.clone();
    assert_eq!(
        run_mut(
            RejoinCaps {
                bytes: 100,
                entries: 10,
                depth: 4
            },
            Box::new(move || std::fs::write(&t1, b"hello-grew").unwrap())
        ),
        Err(RejoinNotice::ChangedUnderfoot)
    );
    assert!(!conv.join(REJOIN_DIR).exists());

    std::fs::write(&target, b"123").unwrap();
    let t2 = target.clone();
    assert_eq!(
        run_mut(
            RejoinCaps {
                bytes: 10,
                entries: 10,
                depth: 4
            },
            Box::new(move || std::fs::write(&t2, b"123456789012345").unwrap())
        ),
        Err(RejoinNotice::TooLarge(10))
    );
    assert!(!conv.join(REJOIN_DIR).exists());

    std::fs::write(&target, b"123").unwrap();
    let t3 = target.clone();
    assert_eq!(
        run_mut(
            RejoinCaps::DEFAULT,
            Box::new(move || {
                let _ = std::fs::remove_file(&t3);
            })
        ),
        Err(RejoinNotice::ChangedUnderfoot)
    );
    assert!(!conv.join(REJOIN_DIR).exists());
    // Replaced inode with identical contents -> same_entry == false.
    std::fs::write(&target, b"123").unwrap();
    let t4 = target.clone();
    assert_eq!(
        run_mut(
            RejoinCaps::DEFAULT,
            Box::new(move || {
                std::fs::remove_file(&t4).unwrap();
                std::fs::write(&t4, b"123").unwrap();
            })
        ),
        Err(RejoinNotice::ChangedUnderfoot)
    );
    assert!(!conv.join(REJOIN_DIR).exists());

    // Same inode and size, modified mtime -> metadata tuple != before.
    std::fs::write(&target, b"123").unwrap();
    let t5 = target.clone();
    assert_eq!(
        run_mut(
            RejoinCaps::DEFAULT,
            Box::new(move || {
                let f = std::fs::OpenOptions::new().write(true).open(&t5).unwrap();
                f.set_modified(
                    std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000),
                )
                .unwrap();
            })
        ),
        Err(RejoinNotice::ChangedUnderfoot)
    );
    assert!(!conv.join(REJOIN_DIR).exists());
    assert_eq!(
        std::fs::read(conv.join("transcript.jsonl")).unwrap(),
        b"keep-me"
    );
}

#[test]
fn parent_errors_and_notice_display_bind() {
    let dir = tempfile::tempdir().unwrap();
    let engine = dir.path().join("engine");
    plant_conversation(&engine, UUID, b"db", &[("a.txt", b"ok")]);

    assert_eq!(
        keep_rejoin(UUID, &engine, None, &[], false, RejoinCaps::DEFAULT),
        Err(RejoinNotice::Parent(ParentError::NoOperatorHome))
    );
    assert_eq!(
        agy_keep::keep(UUID, &engine, None, &[]),
        Err(KeepNotice::NoOperatorHome)
    );
    assert_eq!(
        keep_rejoin(
            UUID,
            &engine,
            Some(Path::new("relative/home")),
            &[],
            false,
            RejoinCaps::DEFAULT
        ),
        Err(RejoinNotice::Parent(ParentError::InvalidReference))
    );
    let non_utf8_home = Path::new(OsStr::from_bytes(b"/tmp/bad-\xff-home"));
    assert_eq!(
        keep_rejoin(
            UUID,
            &engine,
            Some(non_utf8_home),
            &[],
            false,
            RejoinCaps::DEFAULT
        ),
        Err(RejoinNotice::Parent(ParentError::NonUtf8Home))
    );
    assert_eq!(
        agy_keep::keep(UUID, &engine, Some(non_utf8_home), &[]),
        Err(KeepNotice::NonUtf8Home)
    );

    let wide_parent_op = dir.path().join("op-wide-parent");
    let wide_parent = kept_parent(&wide_parent_op);
    std::fs::create_dir_all(&wide_parent).unwrap();
    std::fs::set_permissions(&wide_parent, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(
        keep_rejoin(
            UUID,
            &engine,
            Some(&wide_parent_op),
            &[],
            false,
            RejoinCaps::DEFAULT
        ),
        Err(RejoinNotice::Parent(ParentError::UnownedParent))
    );

    let wide_op = dir.path().join("op-wide");
    let wide_conv = prep_kept_transcript(&wide_op, UUID, b"t");
    std::fs::set_permissions(&wide_conv, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(
        keep_rejoin(
            UUID,
            &engine,
            Some(&wide_op),
            &[],
            true,
            RejoinCaps::DEFAULT
        ),
        Err(RejoinNotice::Parent(ParentError::UnownedParent))
    );
}

#[test]
fn rejoin_notice_display_messages_bind() {
    let perm = std::io::Error::from(ErrorKind::PermissionDenied);
    let other = std::io::Error::from(ErrorKind::Other);
    let notices: [(RejoinNotice, String); 19] = [
        (
            RejoinNotice::InvalidId,
            "agy rejoin state not kept: init's conversation id is not a lowercase UUID".into(),
        ),
        (
            RejoinNotice::Parent(ParentError::NoOperatorHome),
            "agy rejoin state not kept: the operator has no HOME to keep it under".into(),
        ),
        (
            RejoinNotice::Parent(ParentError::InvalidReference),
            "agy rejoin state not kept: the reference does not pass the run view's reference rule"
                .into(),
        ),
        (
            RejoinNotice::Parent(ParentError::NonUtf8Home),
            "agy rejoin state not kept: the kept-conversations directory path is not UTF-8".into(),
        ),
        (
            RejoinNotice::Parent(ParentError::UnownedParent),
            "agy rejoin state not kept: the kept-conversations parent is not an owner-only directory of this user's"
                .into(),
        ),
        (
            RejoinNotice::Parent(ParentError::ParentIo(ErrorKind::PermissionDenied)),
            format!(
                "agy rejoin state not kept: could not prepare the kept-conversations parent: {perm}"
            ),
        ),
        (
            RejoinNotice::Parent(ParentError::ReplacedDirectory),
            "agy rejoin state not kept: a newly created kept directory was replaced before it could be opened"
                .into(),
        ),
        (
            RejoinNotice::ReferenceMasked,
            "agy rejoin state not kept: masking the turn's secret bindings would rewrite the root session row"
                .into(),
        ),
        (
            RejoinNotice::BoundValueAppeared,
            "agy rejoin state not kept: a bound secret value appeared in the conversation".into(),
        ),
        (
            RejoinNotice::Source(SourceErrno::Absent),
            "agy rejoin state not kept: a required conversation file or directory is absent".into(),
        ),
        (
            RejoinNotice::Source(SourceErrno::Unsafe),
            "agy rejoin state not kept: a path component under the engine HOME is a symbolic link or not of the expected file type"
                .into(),
        ),
        (
            RejoinNotice::Source(SourceErrno::Io(ErrorKind::PermissionDenied)),
            format!("agy rejoin state not kept: could not read source: {perm}"),
        ),
        (
            RejoinNotice::TooLarge(128),
            "agy rejoin state not kept: conversation state exceeds the 128-byte copy limit".into(),
        ),
        (
            RejoinNotice::TooManyEntries(4),
            "agy rejoin state not kept: conversation state exceeds the 4-entry limit".into(),
        ),
        (
            RejoinNotice::TooDeep(2),
            "agy rejoin state not kept: conversation brain exceeds the 2-level depth limit".into(),
        ),
        (
            RejoinNotice::EmptyBrain,
            "agy rejoin state not kept: conversation brain directory is empty".into(),
        ),
        (
            RejoinNotice::ChangedUnderfoot,
            "agy rejoin state not kept: a conversation file changed while it was being copied"
                .into(),
        ),
        (
            RejoinNotice::AlreadyKept,
            "agy rejoin state not kept: rejoin state for this conversation already exists".into(),
        ),
        (
            RejoinNotice::Io(ErrorKind::Other),
            format!("agy rejoin state not kept: could not copy rejoin state: {other}"),
        ),
    ];
    for (notice, expected) in notices {
        assert_eq!(notice.to_string(), expected);
    }
}

#[test]
fn rollback_refuses_replaced_created_directory() {
    let dir = tempfile::tempdir().unwrap();
    let swap_op = dir.path().join("op-swap");
    let swap_parent = kept_parent(&swap_op);
    let parent_fd = agy_keep::owned_parent_raw(&swap_parent).unwrap();
    let target_dir = swap_parent.join(UUID);
    let moved_dir = swap_parent.join("moved-aside");
    let sentinel = target_dir.join("keep-replacement");
    let mut replaced_id = None;
    let mut rollback = RejoinRollback::begin(parent_fd);
    assert_eq!(
        rollback
            .create_root_dir_with(OsStr::new(UUID), || {
                std::fs::rename(&target_dir, &moved_dir).unwrap();
                std::fs::create_dir(&target_dir).unwrap();
                std::fs::write(&sentinel, b"owner").unwrap();
                let meta = std::fs::symlink_metadata(&target_dir).unwrap();
                replaced_id = Some((meta.dev(), meta.ino()));
            })
            .map(|_| ()),
        Err(RejoinNotice::Parent(ParentError::ReplacedDirectory))
    );
    drop(rollback);
    assert!(target_dir.is_dir());
    let after = std::fs::symlink_metadata(&target_dir).unwrap();
    assert_eq!(Some((after.dev(), after.ino())), replaced_id);
    assert_eq!(std::fs::read(&sentinel).unwrap(), b"owner");

    // Mechanism (m4) on nested rollback descent: if a created subdirectory is
    // replaced before Drop, rollback skips descending into the replacement.
    let nested_op = dir.path().join("op-nested-swap");
    let nested_parent = kept_parent(&nested_op);
    let nested_fd = agy_keep::owned_parent_raw(&nested_parent).unwrap();
    let mut nested = RejoinRollback::begin(nested_fd);
    let (conv_fd, conv_node) = nested
        .create_root_dir_with(OsStr::new(UUID), || {})
        .unwrap();
    let (sub_fd, sub_node) = nested
        .create_dir(
            std::os::fd::AsFd::as_fd(&conv_fd),
            conv_node,
            OsStr::new("rejoin"),
        )
        .unwrap();
    let file_fd = nested
        .create_file(
            std::os::fd::AsFd::as_fd(&sub_fd),
            sub_node,
            OsStr::new("conversation.db"),
        )
        .unwrap();
    drop((file_fd, sub_fd, conv_fd));
    let created_rejoin = nested_parent.join(UUID).join("rejoin");
    let moved_rejoin = dir.path().join("moved-rejoin");
    std::fs::rename(&created_rejoin, &moved_rejoin).unwrap();
    std::fs::create_dir(&created_rejoin).unwrap();
    std::fs::write(created_rejoin.join("conversation.db"), b"keep-replaced").unwrap();
    drop(nested);
    assert_eq!(
        std::fs::read(created_rejoin.join("conversation.db")).unwrap(),
        b"keep-replaced"
    );
}

#[test]
fn version_observer_and_refused_to_start_bind() {
    let vdir = tempfile::tempdir().unwrap();
    let make_v = |name: &str, body: &str| {
        let p = vdir.path().join(name);
        std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        p
    };
    let wd = vdir.path().to_str().unwrap();
    let owned = [("AGY_CLI_DISABLE_AUTO_UPDATE", "true")];
    let ver_cmd = |bin: &str| [bin.to_string(), "--version".to_string()];
    assert_eq!(
        observed_version_in(
            &ver_cmd(make_v("ok", "printf '1.5.14\\n'").to_str().unwrap()),
            wd,
            &owned
        ),
        Some("1.5.14".into())
    );
    assert_eq!(
        observed_version_in(
            &ver_cmd(make_v("fail", "exit 1").to_str().unwrap()),
            wd,
            &owned
        ),
        None
    );
    assert_eq!(
        observed_version_in(
            &ver_cmd(make_v("bad_utf8", "printf '\\377\\n'").to_str().unwrap()),
            wd,
            &owned
        ),
        None
    );
    assert_eq!(
        observed_version_in(
            &ver_cmd(make_v("words", "printf 'no-version\\n'").to_str().unwrap()),
            wd,
            &owned
        ),
        None
    );
    assert_eq!(
        observed_version_in(&ver_cmd("/no/such/agy-bin"), wd, &owned),
        None
    );
    assert_eq!(observed_version_in(&[], wd, &owned), None);

    // Refused-to-start turn with declared root emits no root_session and keeps no rejoin state.
    let mut outcome = Err(super::super::Ended::Driver(String::new()));
    let mut saw_harness_started = false;
    let conv = super::super::fold::Conversation::Named(UUID.into());
    super::super::rejoin::publish_turn(
        super::super::rejoin::TurnKeep {
            declared: super::super::rejoin::DeclaredKinds {
                transcript: true,
                root: true,
            },
            probe: false,
            refused_to_start: true,
            pin_confirmed: true,
            conversation: &conv,
            harness_version: Some("1.5.14"),
            engine_home: vdir.path(),
            operator_home: Some(vdir.path()),
            bindings: &[],
        },
        &mut outcome,
        &mut |e| {
            if matches!(e.step, crate::adapters::Step::HarnessStarted) {
                saw_harness_started = true;
            }
        },
    );
    assert!(!saw_harness_started);
}

fn run_declared_work(
    version: (&str, i32),
    db: Option<&[u8]>,
    secret_binding: Option<(&str, &str)>,
    stdout: &str,
) -> (World, PathBuf, Vec<crate::Body>) {
    let mut world = World::new();
    let seat_dir = world.seat();
    let bin = write_rejoin_fake_agy(&seat_dir, version, UUID, db, Some(b"step-text"), stdout);
    let mut input = declared_work_input(&world);
    if let Some((name, val)) = secret_binding {
        let store = world.path("secrets.env");
        secret::store_set(&store, name, val).unwrap();
        input["secrets"] = json!([name]);
        input["secrets_file"] = json!(store);
    }
    let messages = run_seat_agy(&mut world, &bin, input, false);
    (world, seat_dir, messages)
}

#[test]
fn work_turn_records_root_session_persistent_iff_kept_whole() {
    let stdout = format!("{}\n{STEP_DONE}\n{OK_RES}\n", init_line(UUID, FLASH));

    let (world, seat_dir, messages) =
        run_declared_work(("1.5.14\n", 0), Some(b"sqlite-db"), None, &stdout);
    assert!(world.homes_gone(), "engine HOME must be removed");
    let env_text = std::fs::read_to_string(seat_dir.join("version.env")).unwrap();
    let env_lines: Vec<&str> = env_text.lines().collect();
    assert_eq!(
        env_lines[0], "true",
        "AGY_CLI_DISABLE_AUTO_UPDATE=true on --version"
    );
    assert_ne!(Path::new(env_lines[1]), world.operator_home());

    let conv = conv_dir(world.operator_home(), UUID);
    assert!(conv.join("transcript.jsonl").exists());
    assert!(conv.join(REJOIN_DIR).join(REJOIN_DB_FILE).exists());
    assert!(conv
        .join(REJOIN_DIR)
        .join(REJOIN_BRAIN_DIR)
        .join(".system_generated/steps/1/output.txt")
        .exists());
    let started = harness_started(&messages).expect("harness-started checkpoint");
    assert_eq!(started["harness"], "agy");
    assert_eq!(started["launch"], "cold");
    assert_eq!(
        started["root_session"],
        json!({"kind": "agy-conversation", "id": UUID, "harness_version": "1.5.14", "persistent": true})
    );
    assert_eq!(started["transcript"]["kind"], "agy-conversation");
    assert_eq!(started["transcript"]["locator"], UUID);
    drop(world);

    // Missing DB -> persistent: false, rejoin absent, transcript STILL kept.
    let (world2, _, messages2) = run_declared_work(("1.5.14\n", 0), None, None, &stdout);
    let conv2 = conv_dir(world2.operator_home(), UUID);
    assert!(conv2.join("transcript.jsonl").exists());
    assert!(!conv2.join(REJOIN_DIR).exists());
    let started2 = harness_started(&messages2).expect("harness-started on missing db");
    assert_eq!(started2["root_session"]["persistent"], json!(false));
    assert!(started2.get("transcript").is_none());
}

#[test]
fn work_turn_handles_bound_secret_version_failure_and_masked_root_session() {
    let stdout = format!("{}\n{STEP_DONE}\n{OK_RES}\n", init_line(UUID, FLASH));

    // Bound value in conversation -> persistent: false, rejoin absent, transcript kept.
    let (w1, _, m1) = run_declared_work(
        ("1.5.14\n", 0),
        Some(b"db with secret-printed-value-42"),
        Some(("MY_KEY", "secret-printed-value-42")),
        &stdout,
    );
    assert!(conv_dir(w1.operator_home(), UUID)
        .join("transcript.jsonl")
        .exists());
    assert!(!conv_dir(w1.operator_home(), UUID).join(REJOIN_DIR).exists());
    assert_eq!(
        harness_started(&m1).unwrap()["root_session"]["persistent"],
        json!(false)
    );
    drop(w1);

    // agy --version fails -> no root_session, no rejoin, transcript kept.
    let (w2, _, m2) = run_declared_work(("", 1), Some(b"db"), None, &stdout);
    assert!(conv_dir(w2.operator_home(), UUID)
        .join("transcript.jsonl")
        .exists());
    assert!(!conv_dir(w2.operator_home(), UUID).join(REJOIN_DIR).exists());
    assert_no_root_session(&m2);
    drop(w2);

    // Bound value masks root_session row ("1.5.14") -> no root_session, no rejoin.
    let (w3, _, m3) = run_declared_work(
        ("1.5.14\n", 0),
        Some(b"db"),
        Some(("VER_S", "1.5.14")),
        &stdout,
    );
    assert!(!conv_dir(w3.operator_home(), UUID).join(REJOIN_DIR).exists());
    assert_no_root_session(&m3);
}

#[test]
fn offered_session_refused_before_spawn_and_non_keeping_seats_emit_no_root_session() {
    let stdout = format!("{}\n{STEP_DONE}\n{OK_RES}\n", init_line(UUID, FLASH));

    // R4: offered session refused before any process starts (including --version).
    {
        let mut world = World::new();
        let marker = world.seat().join("spawned.marker");
        let spy = world.seat().join("agy");
        std::fs::write(
            &spy,
            format!("#!/bin/sh\n: > '{}'\nexit 0\n", marker.display()),
        )
        .unwrap();
        std::fs::set_permissions(&spy, std::fs::Permissions::from_mode(0o755)).unwrap();
        world.set(&spy);
        let input = declared_work_input(&world);
        let err = super::drive_flash(
            &spy,
            StartMode {
                offered: true,
                probe: false,
            },
            &input,
            &[],
        )
        .err()
        .expect("offered session refused");
        assert_eq!(err, LaunchError::Resumed.to_string());
        assert!(!marker.exists(), "offered session must spawn no process");
    }

    // R5: gate / undeclared-root keeps no rejoin and emits no root_session.
    {
        let mut world = World::new();
        let bin = write_rejoin_fake_agy(
            &world.seat(),
            ("1.5.14\n", 0),
            UUID,
            Some(b"db"),
            Some(b"ok"),
            &stdout,
        );
        let mut input = world.input(None);
        input["resume_context"] =
            json!({"assessment": Value::Null, "declared_transcript_kind": "agy-conversation"});
        let msgs = run_seat_agy(&mut world, &bin, input, false);
        assert!(!conv_dir(world.operator_home(), UUID)
            .join(REJOIN_DIR)
            .exists());
        assert_no_root_session(&msgs);
    }

    // R5: probe keeps nothing and emits no notice.
    {
        let mut world = World::new();
        let bin = write_rejoin_fake_agy(
            &world.seat(),
            ("1.5.14\n", 0),
            UUID,
            Some(b"db"),
            Some(b"ok"),
            &stdout,
        );
        world.set(&bin);
        let input = declared_work_input(&world);
        let inv = super::drive_flash(
            &bin,
            StartMode {
                offered: false,
                probe: true,
            },
            &input,
            &[],
        )
        .expect("probe succeeds");
        assert_eq!(inv.stderr, "");
        assert!(!conv_dir(world.operator_home(), UUID).exists());
    }
}

#[test]
fn bad_or_absent_conversation_id_and_invalid_declared_root_kind_bind() {
    for bad_stdout in [
        format!(
            "{}\n{STEP_DONE}\n{OK_RES}\n",
            init_line("not-a-uuid", FLASH)
        ),
        format!("{STEP_DONE}\n{OK_RES}\n"),
    ] {
        let (world, _, msgs) = run_declared_work(("1.5.14\n", 0), Some(b"db"), None, &bad_stdout);
        assert!(!conv_dir(world.operator_home(), UUID).exists());
        assert_no_root_session(&msgs);
    }

    let invalid_root = crate::declared_transcript::InvalidDeclaredRootSessionKind;
    assert_eq!(
        invalid_root.to_string(),
        "resume_context.declared_root_session_kind is not a valid declared root session kind"
    );
    for bad_ctx in [
        json!({"declared_root_session_kind": 42}),
        json!({"declared_root_session_kind": "none"}),
        json!({"declared_root_session_kind": "AGY"}),
        json!("not-an-object"),
    ] {
        assert_eq!(
            DeclaredTranscriptKind::read_root_session_from(&json!({"resume_context": bad_ctx})),
            Err(invalid_root)
        );
    }

    let mut world = World::new();
    let bin = world.seat().join("agy");
    world.set(&bin);
    let mut bad_input = world.input(None);
    bad_input["resume_context"] = json!({"declared_root_session_kind": "none"});
    let launch_err = super::drive_flash(&bin, StartMode::default(), &bad_input, &[])
        .err()
        .expect("bad declared_root_session_kind refused");
    assert_eq!(
        launch_err,
        LaunchError::DeclaredRootSessionKind(invalid_root).to_string()
    );
    assert_eq!(
        launch_err,
        "refusing to invoke the agent CLI: resume_context.declared_root_session_kind is not a valid declared root session kind"
    );
    assert!(world.homes_gone());
}

#[test]
fn empty_brain_is_refused_and_rolled_back() {
    let dir = tempfile::tempdir().unwrap();
    let (engine, op) = (dir.path().join("empty-brain"), dir.path().join("op"));
    std::fs::create_dir_all(&op).unwrap();
    plant_conversation(&engine, UUID, b"db", &[]);
    assert_eq!(
        keep_rejoin(UUID, &engine, Some(&op), &[], false, RejoinCaps::DEFAULT),
        Err(RejoinNotice::EmptyBrain)
    );
    assert!(!conv_dir(&op, UUID).exists());
}

fn expect_configured_refusal(world: &mut World, bin: &Path, expected_path: &Path) {
    world.set(bin);
    let input = declared_work_input(world);
    let err = super::drive_flash(bin, StartMode::default(), &input, &[])
        .err()
        .expect("workspace config must refuse launch");
    assert_eq!(
        err,
        LaunchError::Configured(expected_path.to_path_buf()).to_string()
    );
    assert!(world.homes_gone());
}

#[test]
fn version_probe_applies_sealed_removals_and_readmits_workspace_before_launch() {
    let mut env = crate::env_guard::EnvGuard::lock();
    env.set("BROKKR_TEST_REJOIN_REMOVED", "must-not-leak");
    let vdir = tempfile::tempdir().unwrap();
    let leaked_path = vdir.path().join("seen.env");
    let spy_bin = write_exec(
        &vdir.path().join("agy-ver"),
        &format!(
            "#!/bin/sh\nprintf '%s' \"${{BROKKR_TEST_REJOIN_REMOVED:-ABSENT}}\" > '{}'\n\
             printf '1.5.14\\n'\n",
            leaked_path.display()
        ),
    );
    let mut serving = sealed(
        crate::native_controls::SealedBoundary::Namespace,
        "empty",
        json!("measured"),
        "standing",
    );
    serving["isolation"]["servers"] = json!({"brokered": [{
        "capability": "library-docs",
        "plan": "/state/plans/library-docs.json",
        "plan_digest": "a".repeat(64),
        "removals": ["BROKKR_TEST_REJOIN_REMOVED"],
    }]});
    let removal_input = json!({ crate::native_controls::SERVING_INPUTS: serving });
    let _removing = crate::adapters::mcp::Removing::attempt(&removal_input);
    let cmd = [spy_bin.to_string_lossy().into_owned(), "--version".into()];
    assert_eq!(
        observed_version_in(
            &cmd,
            vdir.path().to_str().unwrap(),
            &[("AGY_CLI_DISABLE_AUTO_UPDATE", "true")],
        ),
        Some("1.5.14".into())
    );
    assert_eq!(std::fs::read_to_string(&leaked_path).unwrap(), "ABSENT");
    drop(_removing);
    drop(env);

    // Config planted during `agy --version` refuses before turn spawn.
    let mut w_during = World::new();
    let seat_during = w_during.seat();
    let turn_marker = seat_during.join("turn-started.marker");
    let bin_during = write_exec(
        &seat_during.join("agy"),
        &format!(
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then\n\
               mkdir -p \"$PWD/.agents\" && : > \"$PWD/.agents/hooks.json\"\n\
               printf '1.5.14\\n'; exit 0\n\
             fi\n: > '{}'; exit 0\n",
            turn_marker.display()
        ),
    );
    let planted_during = seat_during
        .canonicalize()
        .unwrap()
        .join(".agents/hooks.json");
    expect_configured_refusal(&mut w_during, &bin_during, &planted_during);
    assert!(!turn_marker.exists(), "model turn must never spawn");
    drop(w_during);

    // Pre-existing workspace config spawns nothing, not even --version.
    let mut w_before = World::new();
    let seat_before = w_before.seat();
    let any_marker = seat_before.join("any-started.marker");
    let bin_before = write_exec(
        &seat_before.join("agy"),
        &format!("#!/bin/sh\n: > '{}'\nexit 0\n", any_marker.display()),
    );
    let planted = plant(&seat_before, ".agents/hooks.json")
        .canonicalize()
        .unwrap();
    expect_configured_refusal(&mut w_before, &bin_before, &planted);
    assert!(!any_marker.exists(), "--version must not spawn");
}

fn make_fifo(path: &Path) {
    use std::ffi::CString;
    let cpath = CString::new(path.as_os_str().as_bytes()).unwrap();
    let rc = unsafe { libc::mkfifo(cpath.as_ptr(), 0o600) };
    assert_eq!(rc, 0, "mkfifo failed: {}", std::io::Error::last_os_error());
}

fn stamp_earlier(path: &Path) {
    let f = std::fs::OpenOptions::new().write(true).open(path).unwrap();
    f.set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000))
        .unwrap();
}

fn assert_post_open_replacement_copies_opened_handle(
    scratch: &Path,
    label: &str,
    trigger: &OsStr,
    rel_src: &str,
    rel_dest: &str,
    (original, replacement): (&[u8], &[u8]),
) {
    let (_d, engine, op, conv) =
        setup_planted_with_transcript(b"db-original", &[("a.txt", b"original")], b"keep-me");
    let src_path = engine.join(rel_src);
    let dest_path = conv.join(REJOIN_DIR).join(rel_dest);
    let pinned = scratch.join(format!("orig-{label}"));
    let (mut replaced, mut copied_bytes) = (false, None);
    let res = keep_rejoin_with(
        UUID,
        &engine,
        Some(&op),
        &[],
        true,
        RejoinCaps::DEFAULT,
        &mut |point| match point {
            CopyPoint::ReadFile(name) if name == trigger && !replaced => {
                replaced = true;
                std::fs::rename(&src_path, &pinned).unwrap();
                std::fs::write(&src_path, replacement).unwrap();
            }
            CopyPoint::FinalVerify => {
                copied_bytes = Some(std::fs::read(&dest_path).unwrap());
            }
            _ => {}
        },
    );
    assert!(replaced, "{label}");
    assert_eq!(
        copied_bytes.as_deref(),
        Some(original),
        "{label}: copy must read from the classified open handle, not reopen by name"
    );
    assert_rolled_back(res, &conv, RejoinNotice::ChangedUnderfoot, label);
}

#[test]
fn file_opened_once_copies_opened_handle_and_catches_later_replacement() {
    let dir = tempfile::tempdir().unwrap();
    let scratch = dir.path().join("scratch");
    std::fs::create_dir_all(&scratch).unwrap();

    // Changes BEFORE the single open are what gets opened and copied
    // (no prior name-based classification exists).
    {
        let (_d, engine, op, conv) =
            setup_planted_with_transcript(b"db-before", &[("a.txt", b"before")], b"keep-me");
        let db_path = engine
            .join(".gemini/antigravity-cli/conversations")
            .join(format!("{UUID}.db"));
        let db_name = OsString::from(format!("{UUID}.db"));
        let brain_file = engine
            .join(".gemini/antigravity-cli/brain")
            .join(UUID)
            .join("a.txt");
        let (mut fired_db, mut fired_a) = (false, false);
        keep_rejoin_with(
            UUID,
            &engine,
            Some(&op),
            &[],
            true,
            RejoinCaps::DEFAULT,
            &mut |point| {
                if let CopyPoint::OpenEntry(name) = point {
                    if name == db_name.as_os_str() && !fired_db {
                        fired_db = true;
                        std::fs::rename(&db_path, scratch.join("before-open-db")).unwrap();
                        std::fs::write(&db_path, b"db-after-open").unwrap();
                    } else if name == OsStr::new("a.txt") && !fired_a {
                        fired_a = true;
                        std::fs::write(&brain_file, b"after-open").unwrap();
                    }
                }
            },
        )
        .unwrap();
        assert!(fired_db && fired_a);
        assert_eq!(
            std::fs::read(conv.join(REJOIN_DIR).join(REJOIN_DB_FILE)).unwrap(),
            b"db-after-open"
        );
        assert_eq!(
            std::fs::read(conv.join(REJOIN_DIR).join(REJOIN_BRAIN_DIR).join("a.txt")).unwrap(),
            b"after-open"
        );
    }

    // Replacing a file by name AFTER the single open with different bytes of
    // the same length: the copy reads the original open handle's bytes (verified
    // before the final check), and the final check then refuses and rolls back.
    let db_name = OsString::from(format!("{UUID}.db"));
    assert_post_open_replacement_copies_opened_handle(
        &scratch,
        "db",
        db_name.as_os_str(),
        &format!(".gemini/antigravity-cli/conversations/{UUID}.db"),
        REJOIN_DB_FILE,
        (b"db-original", b"db-reopened"),
    );
    assert_post_open_replacement_copies_opened_handle(
        &scratch,
        "brain-file",
        OsStr::new("a.txt"),
        &format!(".gemini/antigravity-cli/brain/{UUID}/a.txt"),
        &format!("{REJOIN_BRAIN_DIR}/a.txt"),
        (b"original", b"reopened"),
    );
}

#[derive(Clone, Copy)]
enum RefusalTrigger<'a> {
    OpenEntry(&'a OsStr),
    ReadFile(&'a OsStr),
    FinalVerify,
}

impl RefusalTrigger<'_> {
    fn matches(self, point: &CopyPoint<'_>) -> bool {
        match (self, point) {
            (Self::OpenEntry(want), CopyPoint::OpenEntry(got)) => want == *got,
            (Self::ReadFile(want), CopyPoint::ReadFile(got)) => want == *got,
            (Self::FinalVerify, CopyPoint::FinalVerify) => true,
            _ => false,
        }
    }
}

fn assert_refused_rejoin(
    (engine, op, conv): (&Path, &Path, &Path),
    bindings: &[crate::secret::BoundSecret],
    trigger: RefusalTrigger<'_>,
    expected: RejoinNotice,
    label: &str,
    mut mutate: impl FnMut(),
) {
    let mut fired = false;
    let res = keep_rejoin_with(
        UUID,
        engine,
        Some(op),
        bindings,
        true,
        RejoinCaps::DEFAULT,
        &mut |point| {
            if trigger.matches(&point) && !fired {
                fired = true;
                mutate();
            }
        },
    );
    assert!(fired, "{label}");
    assert_rolled_back(res, conv, expected, label);
}

fn assert_rolled_back(
    res: Result<(), RejoinNotice>,
    conv: &Path,
    expected: RejoinNotice,
    label: &str,
) {
    assert_eq!(res, Err(expected), "{label}");
    assert!(!conv.join(REJOIN_DIR).exists(), "{label}");
    assert_eq!(
        std::fs::read(conv.join("transcript.jsonl")).unwrap(),
        b"keep-me",
        "{label}"
    );
}

#[test]
fn unsafe_before_open_and_replacement_after_open_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let scratch = dir.path().join("scratch");
    std::fs::create_dir_all(&scratch).unwrap();

    // Unsafe replacement before the single open (FIFO in brain, directory at db)
    // is refused by fstat on the opened handle without blocking or leaking.
    {
        let (_d, engine, op, conv) =
            setup_planted_with_transcript(b"db12", &[("a.txt", b"hello")], b"keep-me");
        let brain_file = engine
            .join(".gemini/antigravity-cli/brain")
            .join(UUID)
            .join("a.txt");
        assert_refused_rejoin(
            (&engine, &op, &conv),
            &[],
            RefusalTrigger::OpenEntry(OsStr::new("a.txt")),
            RejoinNotice::Source(SourceErrno::Unsafe),
            "fifo-before-open",
            || {
                std::fs::remove_file(&brain_file).unwrap();
                make_fifo(&brain_file);
            },
        );
    }
    {
        let (_d, engine, op, conv) =
            setup_planted_with_transcript(b"db12", &[("a.txt", b"hello")], b"keep-me");
        let db_path = engine
            .join(".gemini/antigravity-cli/conversations")
            .join(format!("{UUID}.db"));
        let db_name = OsString::from(format!("{UUID}.db"));
        assert_refused_rejoin(
            (&engine, &op, &conv),
            &[],
            RefusalTrigger::OpenEntry(db_name.as_os_str()),
            RejoinNotice::Source(SourceErrno::Unsafe),
            "dir-db-before-open",
            || {
                std::fs::remove_file(&db_path).unwrap();
                std::fs::create_dir(&db_path).unwrap();
            },
        );
    }

    // Regular-file replacement of the database AFTER open (before final check):
    // rename the old file into scratch so its inode stays allocated; the final
    // check inspects the database first with no parent directory check ahead of it.
    {
        let (_d, engine, op, conv) =
            setup_planted_with_transcript(b"db12", &[("a.txt", b"hello")], b"keep-me");
        let kept_old = scratch.join("pinned-replaced-regular");
        let db_path = engine.join(format!(".gemini/antigravity-cli/conversations/{UUID}.db"));
        assert_refused_rejoin(
            (&engine, &op, &conv),
            &[],
            RefusalTrigger::FinalVerify,
            RejoinNotice::ChangedUnderfoot,
            "replaced-regular-db",
            || {
                std::fs::rename(&db_path, &kept_old).unwrap();
                std::fs::write(&db_path, b"db12").unwrap();
            },
        );
    }
}

#[test]
fn final_check_refuses_database_replaced_with_directory_at_type_check() {
    let (_d, engine, op, conv) =
        setup_planted_with_transcript(b"db12", &[("a.txt", b"hello")], b"keep-me");
    let db_name = OsString::from(format!("{UUID}.db"));
    let mut fired = false;
    // The final check inspects `<uuid>.db` first via `same_stamp_at`. Flipping
    // only `actual.st_mode`'s file type keeps `(st_dev, st_ino, st_size, mtime)`
    // identical, so only the file-type comparison in `same_stamp` can refuse.
    let res = keep_rejoin_with(
        UUID,
        &engine,
        Some(&op),
        &[],
        true,
        RejoinCaps::DEFAULT,
        &mut |point| {
            if let CopyPoint::FinalStat(name, actual) = point {
                if name == db_name.as_os_str() && !fired {
                    fired = true;
                    actual.st_mode = (actual.st_mode & !0o170_000) | 0o040_000;
                    assert_eq!(
                        rustix::fs::FileType::from_raw_mode(actual.st_mode),
                        rustix::fs::FileType::Directory
                    );
                }
            }
        },
    );
    assert!(fired, "db-type-changed-in-final-stat");
    assert_rolled_back(
        res,
        &conv,
        RejoinNotice::ChangedUnderfoot,
        "db-type-changed-in-final-stat",
    );
}

#[test]
fn same_stamp_refuses_file_type_change_when_other_fields_match() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("entry");
    std::fs::write(&path, b"same").unwrap();
    let file = std::fs::File::open(&path).unwrap();
    let expected = rustix::fs::fstat(&file).unwrap();
    let mut as_dir = expected;
    as_dir.st_mode = (as_dir.st_mode & !0o170_000) | 0o040_000;
    assert_eq!(
        rustix::fs::FileType::from_raw_mode(as_dir.st_mode),
        rustix::fs::FileType::Directory
    );
    assert!(same_stamp(&expected, &expected));
    assert!(!same_stamp(&expected, &as_dir));
}

#[test]
fn final_check_refuses_removed_brain_directory_at_open() {
    let (_d, engine, op, conv) = setup_planted_with_transcript(
        b"db",
        &[("sub/file.txt", b"ok"), ("keep.txt", b"ok")],
        b"keep-me",
    );
    let brain = engine.join(".gemini/antigravity-cli/brain").join(UUID);
    // Database is untouched so `same_stamp_at` passes and `verify_dir_tree`
    // hits its `openat` refusal first, answering `ChangedUnderfoot`.
    assert_refused_rejoin(
        (&engine, &op, &conv),
        &[],
        RefusalTrigger::FinalVerify,
        RejoinNotice::ChangedUnderfoot,
        "removed-brain-root",
        || {
            std::fs::remove_dir_all(&brain).unwrap();
        },
    );
}

type DirMutator = fn(&Path, &Path);

fn swap_sub(brain: &Path, scratch: &Path) {
    let sub = brain.join("sub");
    std::fs::rename(&sub, scratch.join("moved")).unwrap();
    std::fs::create_dir(&sub).unwrap();
    std::fs::write(sub.join("file.txt"), b"ok").unwrap();
}

#[test]
fn source_directory_substituted_removed_or_added_during_walk_refuses_and_rolls_back() {
    let dir = tempfile::tempdir().unwrap();
    let cases: [(&str, DirMutator); 5] = [
        ("substitute-subtree", swap_sub),
        ("add-entry", |b, _| {
            std::fs::write(b.join("sub/extra.txt"), b"new").unwrap();
        }),
        ("remove-entry", |b, _| {
            std::fs::remove_file(b.join("sub/file.txt")).unwrap();
        }),
        ("rename-entry", |b, _| {
            std::fs::rename(b.join("sub/file.txt"), b.join("sub/other.txt")).unwrap();
        }),
        ("substitute-brain-root", |b, s| {
            std::fs::rename(b, s.join("moved-brain")).unwrap();
            std::fs::create_dir_all(b.join("sub")).unwrap();
            std::fs::write(b.join("sub/file.txt"), b"ok").unwrap();
            std::fs::write(b.join("z-keep.txt"), b"ok").unwrap();
        }),
    ];

    for (label, mutate) in cases {
        let root = dir.path().join(label);
        let (engine, op, scratch) = (root.join("eng"), root.join("op"), root.join("tmp"));
        std::fs::create_dir_all(&scratch).unwrap();
        plant_conversation(
            &engine,
            UUID,
            b"db",
            &[("sub/file.txt", b"ok"), ("z-keep.txt", b"ok")],
        );
        let conv = prep_kept_transcript(&op, UUID, b"keep-me");
        let brain = engine.join(".gemini/antigravity-cli/brain").join(UUID);
        // `sub` sorts before `z-keep.txt`, so each mutation fires while the
        // walk is still copying `z-keep.txt`.
        assert_refused_rejoin(
            (&engine, &op, &conv),
            &[],
            RefusalTrigger::ReadFile(OsStr::new("z-keep.txt")),
            RejoinNotice::ChangedUnderfoot,
            label,
            || mutate(&brain, &scratch),
        );
    }
}

#[test]
fn final_check_catches_changes_during_walk_after_subtree_left_and_database_rewrite() {
    type WholeCase = (
        &'static str,
        Option<(&'static str, &'static str)>,
        fn(&Path, &Path),
    );
    let cases: [WholeCase; 5] = [
        ("nested-entry-removed", None, |brain, _| {
            std::fs::remove_file(brain.join("sub/deep/file.txt")).unwrap();
        }),
        ("nested-entry-added", None, |brain, _| {
            std::fs::write(brain.join("sub/deep/added.txt"), b"late").unwrap();
        }),
        (
            "nested-entry-added-with-bound-value",
            Some(("LATE_SECRET", "late-bound-secret-99")),
            |brain, _| {
                std::fs::write(brain.join("sub/deep/leak.txt"), b"late-bound-secret-99").unwrap();
            },
        ),
        ("nested-file-rewritten-in-place", None, |brain, _| {
            std::fs::write(brain.join("sub/deep/file.txt"), b"xxxx").unwrap();
        }),
        ("database-rewritten-in-place", None, |_, db| {
            std::fs::write(db, b"db99").unwrap();
        }),
    ];

    for (label, secret_binding, mutate) in cases {
        let (_dir, engine, op, conv) = setup_planted_with_transcript(
            b"db12",
            &[("sub/deep/file.txt", b"orig"), ("z-after.txt", b"keep")],
            b"keep-me",
        );
        let brain = engine.join(".gemini/antigravity-cli/brain").join(UUID);
        let db = engine
            .join(".gemini/antigravity-cli/conversations")
            .join(format!("{UUID}.db"));
        stamp_earlier(&brain.join("sub/deep/file.txt"));
        stamp_earlier(&db);
        let bindings: Vec<_> = secret_binding
            .map(|(k, v)| bound(k, v))
            .into_iter()
            .collect();
        // `sub` sorts before `z-after.txt`, so each mutation fires DURING the
        // walk while `z-after.txt` is still being copied, after `db` and
        // `sub/deep` have already been copied.
        assert_refused_rejoin(
            (&engine, &op, &conv),
            &bindings,
            RefusalTrigger::ReadFile(OsStr::new("z-after.txt")),
            RejoinNotice::ChangedUnderfoot,
            label,
            || mutate(&brain, &db),
        );
    }
}

#[test]
fn root_session_requires_confirmed_model_pin_even_on_turn_error() {
    let err_res = r#"{"event":"result","result":{"status":"ERROR","error":"turn failed"}}"#;

    // Init naming another model then an error -> no rejoin state, no root_session.
    let wrong_stdout = format!("{}\n{err_res}\n", init_line(UUID, "other-model"));
    let (w_wrong, _, m_wrong) =
        run_declared_work(("1.5.14\n", 0), Some(b"sqlite-db"), None, &wrong_stdout);
    assert!(!conv_dir(w_wrong.operator_home(), UUID)
        .join(REJOIN_DIR)
        .exists());
    assert_no_root_session(&m_wrong);
    drop(w_wrong);

    // Init with no model then an error -> no rejoin state, no root_session.
    let no_model_stdout = format!(r#"{{"event":"init","conversation_id":"{UUID}","init":{{}}}}"#)
        + "\n"
        + err_res
        + "\n";
    let (w_none, _, m_none) =
        run_declared_work(("1.5.14\n", 0), Some(b"sqlite-db"), None, &no_model_stdout);
    assert!(!conv_dir(w_none.operator_home(), UUID)
        .join(REJOIN_DIR)
        .exists());
    assert_no_root_session(&m_none);
    drop(w_none);

    // Init confirming the pin then an error -> rejoin state IS kept and
    // root_session is recorded persistent: true (failed/killed pinned turns
    // are rejoinable).
    let ok_init_err = format!("{}\n{err_res}\n", init_line(UUID, FLASH));
    let (w_pin, _, m_pin) =
        run_declared_work(("1.5.14\n", 0), Some(b"sqlite-db"), None, &ok_init_err);
    let conv_pin = conv_dir(w_pin.operator_home(), UUID);
    assert!(conv_pin.join(REJOIN_DIR).join(REJOIN_DB_FILE).exists());
    assert!(conv_pin
        .join(REJOIN_DIR)
        .join(REJOIN_BRAIN_DIR)
        .join(".system_generated/steps/1/output.txt")
        .exists());
    let started = harness_started(&m_pin).expect("harness-started on pinned error turn");
    assert_eq!(
        started["root_session"],
        json!({"kind": "agy-conversation", "id": UUID, "harness_version": "1.5.14", "persistent": true})
    );
    assert_eq!(started["transcript"]["locator"], UUID);
}

#[test]
fn keep_and_rollback_many_step_directories_under_low_nofile_limit() {
    if !crate::process::in_its_own_engine(
        "adapters::agy::tests::rejoin::keep_and_rollback_many_step_directories_under_low_nofile_limit",
    ) {
        return;
    }
    let mut limit = rustix::process::getrlimit(rustix::process::Resource::Nofile);
    limit.current = Some(256);
    rustix::process::setrlimit(rustix::process::Resource::Nofile, limit).unwrap();

    let dir = tempfile::tempdir().unwrap();
    let engine = dir.path().join("engine");
    let step_paths: Vec<String> = (1..=300)
        .map(|i| format!(".system_generated/steps/{i}/output.txt"))
        .collect();
    let files: Vec<(&str, &[u8])> = step_paths
        .iter()
        .map(|p| (p.as_str(), b"step".as_slice()))
        .collect();
    plant_conversation(&engine, UUID, b"sqlite-db", &files);

    let op_ok = dir.path().join("op-ok");
    std::fs::create_dir_all(&op_ok).unwrap();
    keep_rejoin(UUID, &engine, Some(&op_ok), &[], false, RejoinCaps::DEFAULT).unwrap();
    let kept_brain = conv_dir(&op_ok, UUID)
        .join(REJOIN_DIR)
        .join(REJOIN_BRAIN_DIR);
    assert_eq!(
        std::fs::read(kept_brain.join(".system_generated/steps/1/output.txt")).unwrap(),
        b"step"
    );
    assert_eq!(
        std::fs::read(kept_brain.join(".system_generated/steps/300/output.txt")).unwrap(),
        b"step"
    );

    let op_rb = dir.path().join("op-rb");
    std::fs::create_dir_all(&op_rb).unwrap();
    let last_step = engine
        .join(".gemini/antigravity-cli/brain")
        .join(UUID)
        .join(".system_generated/steps/300/output.txt");
    let mut fired = false;
    let res = keep_rejoin_with(
        UUID,
        &engine,
        Some(&op_rb),
        &[],
        false,
        RejoinCaps::DEFAULT,
        &mut |point| {
            if matches!(point, CopyPoint::FinalVerify) && !fired {
                fired = true;
                std::fs::write(&last_step, b"step-mutated").unwrap();
            }
        },
    );
    assert!(fired);
    assert_eq!(res, Err(RejoinNotice::ChangedUnderfoot));
    assert!(!conv_dir(&op_rb, UUID).exists());
}

#[test]
fn bound_value_in_brain_file_or_dir_name_refuses_rejoin_and_spares_transcript() {
    let secret_val = "name-sentinel-9a4b7c";
    let hex_secret: String = secret_val.bytes().map(|b| format!("{b:02x}")).collect();
    let cases = [
        ("file-raw", format!("note-{secret_val}.txt")),
        ("dir-raw", format!("dir-{secret_val}/clean.txt")),
        ("file-hex", format!("encoded-{hex_secret}.txt")),
    ];
    for (label, rel_path) in cases {
        let (_dir, engine, op, conv) =
            setup_planted_with_transcript(b"clean-db", &[(&rel_path, b"clean-content")], b"keep");
        assert_eq!(
            keep_rejoin(
                UUID,
                &engine,
                Some(&op),
                &[bound("NAME_SECRET", secret_val)],
                true,
                RejoinCaps::DEFAULT,
            ),
            Err(RejoinNotice::BoundValueAppeared),
            "{label}"
        );
        assert!(!conv.join(REJOIN_DIR).exists(), "{label}");
        assert_eq!(
            std::fs::read(conv.join("transcript.jsonl")).unwrap(),
            b"keep",
            "{label}"
        );
    }
}

#[test]
fn listing_setup_fails_on_non_directory_fd() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("not-a-dir.txt");
    std::fs::write(&file, b"plain").unwrap();
    let fd: std::os::fd::OwnedFd = std::fs::File::open(&file).unwrap().into();
    assert_eq!(
        for_each_dir_entry(&fd, |_| Ok(())),
        Err(RejoinNotice::Io(ErrorKind::NotADirectory))
    );
}

#[test]
fn listing_iteration_fails_on_entry_errno() {
    let mut visited = Vec::new();
    let entries = [
        Ok(OsString::from(".")),
        Ok(OsString::from("..")),
        Ok(OsString::from("first.txt")),
        Err(rustix::io::Errno::ACCESS),
    ];
    let err = for_each_listed_entry(entries, |name| {
        visited.push(name);
        Ok(())
    });
    assert_eq!(err, Err(RejoinNotice::Io(ErrorKind::PermissionDenied)));
    assert_eq!(visited, vec![OsString::from("first.txt")]);
}

#[test]
fn open_existing_conversation_dir_fails_when_absent() {
    let dir = tempfile::tempdir().unwrap();
    let (engine, op) = (dir.path().join("engine"), dir.path().join("op"));
    plant_conversation(&engine, UUID, b"db", &[("a.txt", b"ok")]);
    let parent = kept_parent(&op);
    std::fs::create_dir_all(&parent).unwrap();
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(
        keep_rejoin(UUID, &engine, Some(&op), &[], true, RejoinCaps::DEFAULT),
        Err(RejoinNotice::Io(ErrorKind::NotFound))
    );
    assert!(!conv_dir(&op, UUID).exists());
}
