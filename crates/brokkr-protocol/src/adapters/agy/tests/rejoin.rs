//! Rejoin state retention and `root_session` tests for `brokkr driver agy`
//! (#1227, decision 0075 level 2 slice 1).

use std::ffi::OsStr;
use std::io::ErrorKind;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{symlink, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use super::judged::bound;
use super::keep::{kept_parent, run_seat_agy, OK_RES, STEP_DONE, UUID};
use super::*;
use crate::adapters::agy::keep::{ParentError, SourceErrno};
use crate::adapters::agy::rejoin::{
    keep_rejoin, keep_rejoin_with, observe_version, RejoinCaps, RejoinNotice, REJOIN_BRAIN_DIR,
    REJOIN_DB_FILE, REJOIN_DIR,
};
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
    assert_eq!(
        keep_rejoin(
            UUID,
            &eng,
            Some(&op),
            &[],
            false,
            RejoinCaps {
                bytes: 100,
                entries: 1,
                depth: 4
            }
        ),
        Err(RejoinNotice::TooManyEntries(1))
    );
    assert!(!conv_dir(&op, UUID).exists());

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

    // Symlink inside brain rolls back already-copied conversation.db.
    let link_brain_file = dir.path().join("link-bf");
    plant_conversation(&link_brain_file, UUID, b"db", &[("good.txt", b"ok")]);
    symlink(
        &decoy,
        link_brain_file
            .join(".gemini/antigravity-cli/brain")
            .join(UUID)
            .join("evil.txt"),
    )
    .unwrap();
    assert_eq!(
        keep_rejoin(
            UUID,
            &link_brain_file,
            Some(&op),
            &[],
            false,
            RejoinCaps::DEFAULT
        ),
        Err(RejoinNotice::Source(SourceErrno::Unsafe))
    );
    assert!(!conv_dir(&op, UUID).exists());

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

    // Read-only conversation directory exercises RejoinGuard io_errno.
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
        keep_rejoin_with(UUID, &engine, Some(&op), &[], true, caps, &mut |name| {
            if name == OsStr::new("a.txt") {
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
    assert_eq!(
        keep_rejoin(
            UUID,
            &engine,
            Some(Path::new(OsStr::from_bytes(b"/tmp/bad-\xff-home"))),
            &[],
            false,
            RejoinCaps::DEFAULT
        ),
        Err(RejoinNotice::Parent(ParentError::NonUtf8Home))
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

    let notices = [
        RejoinNotice::InvalidId,
        RejoinNotice::Parent(ParentError::NoOperatorHome),
        RejoinNotice::Parent(ParentError::InvalidReference),
        RejoinNotice::Parent(ParentError::NonUtf8Home),
        RejoinNotice::Parent(ParentError::UnownedParent),
        RejoinNotice::Parent(ParentError::ParentIo(ErrorKind::PermissionDenied)),
        RejoinNotice::Parent(ParentError::ReplacedDirectory),
        RejoinNotice::ReferenceMasked,
        RejoinNotice::BoundValueAppeared,
        RejoinNotice::Source(SourceErrno::Absent),
        RejoinNotice::Source(SourceErrno::Unsafe),
        RejoinNotice::Source(SourceErrno::Io(ErrorKind::PermissionDenied)),
        RejoinNotice::TooLarge(128),
        RejoinNotice::TooManyEntries(4),
        RejoinNotice::TooDeep(2),
        RejoinNotice::ChangedUnderfoot,
        RejoinNotice::AlreadyKept,
        RejoinNotice::Io(ErrorKind::Other),
    ];
    for notice in notices {
        assert!(
            notice
                .to_string()
                .starts_with("agy rejoin state not kept: "),
            "{notice}"
        );
    }
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
    assert_eq!(
        observe_version(
            make_v("ok", "printf '1.5.14\\n'").to_str().unwrap(),
            wd,
            &owned
        ),
        Some("1.5.14".into())
    );
    assert_eq!(
        observe_version(make_v("fail", "exit 1").to_str().unwrap(), wd, &owned),
        None
    );
    assert_eq!(
        observe_version(
            make_v("bad_utf8", "printf '\\377\\n'").to_str().unwrap(),
            wd,
            &owned
        ),
        None
    );
    assert_eq!(
        observe_version(
            make_v("words", "printf 'no-version\\n'").to_str().unwrap(),
            wd,
            &owned
        ),
        None
    );
    assert_eq!(observe_version("/no/such/agy-bin", wd, &owned), None);

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
        assert!(err.contains("declares no resume"), "{err}");
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

    for bad_ctx in [
        json!({"declared_root_session_kind": 42}),
        json!({"declared_root_session_kind": "none"}),
        json!({"declared_root_session_kind": "AGY"}),
        json!("not-an-object"),
    ] {
        let err =
            DeclaredTranscriptKind::read_root_session_from(&json!({"resume_context": bad_ctx}))
                .unwrap_err();
        assert!(
            err.to_string()
                .contains("resume_context.declared_root_session_kind"),
            "{err}"
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
    assert!(
        launch_err.contains("resume_context.declared_root_session_kind"),
        "{launch_err}"
    );
    assert!(world.homes_gone());
}
