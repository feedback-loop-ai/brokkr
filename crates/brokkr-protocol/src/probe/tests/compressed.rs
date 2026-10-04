//! dsh's session log, which dsh writes zstd-compressed
//! (`session.v3.jsonl.zstd`), read through a pure-Rust decoder under a
//! bound (#484, the operator's ruling of 2026-10-04): the controller's
//! skeleton, compressed by the zstd CLI, measures what the same bytes
//! written plain measure, and a log that is malformed or decompresses
//! past the bound refuses the probe.

use std::io::ErrorKind;

use serde_json::{json, Value};

use super::observe::{packing, unpacked, Packing};
use super::*;

/// The controller's dsh skeleton, and the same bytes compressed by the
/// zstd CLI 1.5.7, as dsh compresses its log.
const SKELETON: &str = include_str!("../measure/streams/dsh-plain.jsonl");
const PACKED: &[u8] = include_bytes!("../measure/streams/dsh-plain.jsonl.zstd");

/// A dsh-like fake that prints the reply and copies `log` to `name` in a
/// session directory, where dsh writes its session log.
fn dsh_writing(log: &Path, name: &str) -> String {
    format!(
        r#"#!/bin/sh
case " $* " in *" --version "*) echo "0.1.5-rc.1"; exit 0 ;; esac
[ -n "$FAKE_TOKEN" ] || {{ echo "error: FAKE_TOKEN is not set" >&2; exit 1; }}
dir="$HOME/.dsh/sessions/workdir/session-1"
mkdir -p "$dir"
cp '{}' "$dir/{name}"
echo "PROBE-OK"
"#,
        log.display()
    )
}

/// The probe of a dsh-like fake writing `bytes` as `name`.
fn probe_writing(world: &World, bytes: &[u8], name: &str) -> Result<Report, ProbeError> {
    let log = world.dir.path().join(format!("log-{name}"));
    std::fs::write(&log, bytes).unwrap();
    let cli = world.fake(&format!("dsh-{name}"), &dsh_writing(&log, name));
    probe_with(
        AdapterKind::Dsh,
        &cli,
        &dsh_declared(),
        &world.bindings,
        DEADLINE,
    )
}

/// The chief's dsh-zstd on 8316ad4d beside its control dsh-plainlog: the
/// compressed log measures every fact the plain one does, its own path
/// named where a fact names the file it was read from.
#[test]
fn dsh_s_compressed_session_log_measures_what_the_same_log_written_plain_measures() {
    if !in_its_own_engine(
        "probe::tests::compressed::dsh_s_compressed_session_log_measures_what_the_same_log_written_plain_measures",
    ) {
        return;
    }
    let world = world();
    let facts = |bytes: &[u8], name: &str| {
        let report = probe_writing(&world, bytes, name).unwrap();
        serde_json::to_value(report).unwrap()["facts"].clone()
    };
    let zstd = facts(PACKED, "session.v3.jsonl.zstd");
    let plain = facts(SKELETON.as_bytes(), "session.v3.jsonl");
    let renamed = zstd.to_string().replace(".jsonl.zstd", ".jsonl");
    assert_eq!(serde_json::from_str::<Value>(&renamed).unwrap(), plain);
    let log = "~/.dsh/sessions/workdir/session-{n}/session.v{n}.jsonl.zstd";
    assert_eq!(
        [&zstd["events"]["evidence"], &zstd["tools"]["value"]],
        [
            &json!(format!("18 events read from {log}")),
            &json!(["<name>"])
        ]
    );
}

/// A compressed log that is not a zstd stream refuses the probe, naming
/// the file, rather than read as nothing written.
#[test]
fn a_compressed_log_the_decoder_cannot_read_refuses_the_probe() {
    if !in_its_own_engine(
        "probe::tests::compressed::a_compressed_log_the_decoder_cannot_read_refuses_the_probe",
    ) {
        return;
    }
    let world = world();
    let error = probe_writing(&world, SKELETON.as_bytes(), "session.v3.jsonl.zstd").unwrap_err();
    assert!(matches!(error, ProbeError::Io { .. }), "{error:?}");
    assert_eq!(
        error.to_string(),
        "could not decompress the transcript \
         ~/.dsh/sessions/workdir/session-1/session.v3.jsonl.zstd under the scratch HOME: \
         BadMagicNumber(2037654139)"
    );
}

/// Each frame is decompressed in turn, and what passes the bound is
/// refused, never read to its end; a frame cut short or not one at all
/// is refused too.
#[test]
fn a_compressed_log_is_read_frame_by_frame_within_its_bound() {
    let skeleton = SKELETON.as_bytes();
    let two = [PACKED, PACKED].concat();
    let len = skeleton.len() as u64;
    assert_eq!(
        unpacked(&two, 2 * len).unwrap(),
        [skeleton, skeleton].concat()
    );
    assert_eq!(unpacked(&[], 0).unwrap(), Vec::<u8>::new());
    let refused = |packed: &[u8], bound| {
        let error = unpacked(packed, bound).unwrap_err();
        (error.kind(), error.to_string())
    };
    let past = |bound: u64| {
        (
            ErrorKind::FileTooLarge,
            format!("it decompresses past {bound} bytes"),
        )
    };
    let cut = "Failed to parse block header: BlockContentReadError(Error { kind: UnexpectedEof, \
               message: \"failed to fill whole buffer\" })";
    assert_eq!(
        [
            refused(&two, 2 * len - 1),
            refused(PACKED, len - 1),
            refused(&PACKED[..PACKED.len() - 8], len),
            refused(skeleton, len),
        ],
        [
            past(2 * len - 1),
            past(len - 1),
            (ErrorKind::Other, cut.to_string()),
            (ErrorKind::Other, "BadMagicNumber(2037654139)".to_string()),
        ]
    );
}

/// A transcript is a `.jsonl` file, or a `.jsonl.zstd` one; no other name
/// is one, a `.zstd` file of another stem among them.
#[test]
fn a_transcript_is_named_jsonl_or_jsonl_zstd() {
    let named = [
        "s.jsonl",
        "session.v3.jsonl.zstd",
        "s.zstd",
        "s.json.zstd",
        "s.json",
        "zstd",
    ]
    .map(|name| packing(Path::new("/h").join(name).as_path()));
    assert_eq!(
        named,
        [
            Some(Packing::Plain),
            Some(Packing::Zstd),
            None,
            None,
            None,
            None
        ]
    );
}
