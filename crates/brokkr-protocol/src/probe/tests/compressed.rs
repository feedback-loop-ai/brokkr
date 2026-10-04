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

/// The chief's z-control on e1f4e977: claude's cost-state row, its
/// `webSearchRequests` set to 1, compressed by the zstd CLI at
/// `--fast=1000`, which stores it as raw literals.
const WEB_SEARCH: &[u8] = include_bytes!("../measure/streams/claude-websearch.jsonl.zstd");

/// The chief's z-corrupt: [`WEB_SEARCH`] with bit 0 of its byte 425
/// flipped, which the decoder alone reads as a count of 0.
fn corrupt() -> Vec<u8> {
    let mut corrupt = WEB_SEARCH.to_vec();
    corrupt[425] ^= 1;
    corrupt
}

/// The probe of a Claude-like fake whose boxed turn writes `transcript`
/// as a compressed session log, or writes none.
fn claude_writing(
    world: &World,
    name: &str,
    transcript: Option<&[u8]>,
) -> Result<Report, ProbeError> {
    claude_writing_as(world, name, "turn.jsonl.zstd", transcript)
}

/// [`claude_writing`], the transcript written as `file`.
fn claude_writing_as(
    world: &World,
    name: &str,
    file: &str,
    transcript: Option<&[u8]>,
) -> Result<Report, ProbeError> {
    let boxed = match transcript {
        Some(bytes) => {
            let log = world.dir.path().join(format!("{name}.log"));
            std::fs::write(&log, bytes).unwrap();
            format!(
                r#"{BOXED_CLEAN}; mkdir -p "$HOME/.claude/projects/workdir"; cp '{}' "$HOME/.claude/projects/workdir/{file}""#,
                log.display()
            )
        }
        None => BOXED_CLEAN.to_string(),
    };
    let cli = world.fake(name, &claude_with("9.9.9", ":", &boxed));
    probe_with(
        AdapterKind::Claude,
        &cli,
        &claude_declared(),
        &world.bindings,
        DEADLINE,
    )
}

/// The chief's z-corrupt on e1f4e977 beside its controls z-control and
/// z-none: a compressed transcript whose frame fails its own checksum
/// refuses the probe, naming the file, where the intact one keeps the
/// boxed turn's tool run in view and no transcript at all reads boxed.
#[test]
fn a_compressed_transcript_failing_its_checksum_refuses_the_probe() {
    if !in_its_own_engine(
        "probe::tests::compressed::a_compressed_transcript_failing_its_checksum_refuses_the_probe",
    ) {
        return;
    }
    let world = world();
    let view = |name: &str, transcript: Option<&[u8]>| {
        let report =
            serde_json::to_value(claude_writing(&world, name, transcript).unwrap()).unwrap();
        (
            report["facts"]["boxed_tools"].clone(),
            report["eligibility"]["verdict"].clone(),
        )
    };
    assert_eq!(
        [view("z-control", Some(WEB_SEARCH)), view("z-none", None)],
        [
            (
                unmeasured(
                    "the turn's tools listed mcp__brokkr__workspace, but the cost-state event \
                     on line 1 of ~/.claude/projects/workdir/turn.jsonl.zstd ran WebSearch at \
                     /modelUsage/*/webSearchRequests"
                ),
                json!("unboxed-only")
            ),
            (
                measured(
                    json!([]),
                    "the system/init event on line 1 of stdout listed tools: 1"
                ),
                json!("boxed")
            ),
        ]
    );
    let error = claude_writing(&world, "z-corrupt", Some(&corrupt())).unwrap_err();
    assert!(matches!(error, ProbeError::Io { .. }), "{error:?}");
    assert_eq!(
        error.to_string(),
        format!(
            "could not decompress the transcript ~/.claude/projects/workdir/turn.jsonl.zstd \
             under the scratch HOME: {MISMATCH}"
        )
    );
}

/// How [`corrupt`]'s one frame is refused.
const MISMATCH: &str = "a frame's bytes do not match its content checksum 0x479ce038";

/// A frame is read only when it states a content checksum and its bytes
/// match it, the second frame of a file as much as the first; a frame
/// stating none, here the skeleton's with its checksum cut and its
/// descriptor's checksum flag cleared, is refused, and so, for now, is a
/// skippable frame.
#[test]
fn a_frame_is_read_only_when_its_content_checksum_vouches_for_its_bytes() {
    const BOUND: u64 = 1 << 20;
    // claude's recorded cost-state row, its web search count raised.
    let recorded = include_str!("../measure/streams/claude-plain.jsonl")
        .lines()
        .nth(23);
    let row = recorded
        .unwrap()
        .replace(r#""webSearchRequests":0"#, r#""webSearchRequests":1"#);
    assert_eq!(
        unpacked(WEB_SEARCH, BOUND).unwrap(),
        format!("{row}\n").into_bytes()
    );
    let mut bare = PACKED[..PACKED.len() - 4].to_vec();
    bare[4] &= !0b100;
    let skippable = [0x50, 0x2a, 0x4d, 0x18, 4, 0, 0, 0, 0, 0, 0, 0];
    let refused = |packed: &[u8]| {
        let error = unpacked(packed, BOUND).unwrap_err();
        (error.kind(), error.to_string())
    };
    let unverified = (
        ErrorKind::InvalidData,
        "a frame carries no content checksum to verify it by".to_string(),
    );
    assert_eq!(
        [
            refused(&corrupt()),
            refused(&bare),
            refused(&[PACKED, &bare].concat()),
            refused(&[PACKED, &skippable].concat()),
        ],
        [
            (ErrorKind::InvalidData, MISMATCH.to_string()),
            unverified.clone(),
            unverified,
            (
                ErrorKind::Other,
                "SkipFrame { magic_number: 407710288, length: 4 }".to_string()
            ),
        ]
    );
}

/// Every single-bit flip of a checksummed frame either refuses or decodes
/// to the bytes the frame held: none decodes to changed bytes, which the
/// decoder alone let through for most flips (the chief's flip count on
/// e1f4e977).
#[test]
fn no_single_bit_flip_of_a_checksummed_frame_decodes_to_changed_bytes() {
    const BOUND: u64 = 1 << 20;
    for packed in [WEB_SEARCH, PACKED] {
        let held = unpacked(packed, BOUND).unwrap();
        let changed: Vec<usize> = (0..packed.len() * 8)
            .filter(|bit| {
                let mut flipped = packed.to_vec();
                flipped[bit / 8] ^= 1 << (bit % 8);
                unpacked(&flipped, BOUND).is_ok_and(|bytes| bytes != held)
            })
            .collect();
        assert_eq!(changed, Vec::<usize>::new());
    }
}

/// The chief's case on 0d79ad7e beside its control z-control: the bytes
/// that keep the boxed turn's web search in view as `turn.jsonl.zstd`
/// refuse the probe, naming the file, as `turn.jsonl.zst` or
/// `turn.ndjson`, where they read as no tool run and admitted boxed.
#[test]
fn a_transcript_in_a_packing_the_probe_does_not_read_refuses_the_probe() {
    if !in_its_own_engine(
        "probe::tests::compressed::a_transcript_in_a_packing_the_probe_does_not_read_refuses_the_probe",
    ) {
        return;
    }
    let world = world();
    let refused = |file: &str| {
        let error = claude_writing_as(&world, file, file, Some(WEB_SEARCH)).unwrap_err();
        assert!(matches!(error, ProbeError::Io { .. }), "{error:?}");
        error.to_string()
    };
    let unread = |file: &str| {
        format!(
            "could not read the transcript ~/.claude/projects/workdir/{file} under the \
             scratch HOME: the probe reads a transcript only as .jsonl or .jsonl.zstd"
        )
    };
    assert_eq!(
        [refused("turn.jsonl.zst"), refused("turn.ndjson")],
        [unread("turn.jsonl.zst"), unread("turn.ndjson")]
    );
}

/// A name marked as a transcript in a packing the probe does not read,
/// another compression, another spelling or another case, is unread; a
/// name not marked as one is no transcript.
#[test]
fn a_name_marked_as_a_transcript_in_another_packing_is_unread() {
    let named = [
        "turn.jsonl.zst",
        "turn.jsonl.gz",
        "turn.ndjson",
        "turn.JSONL",
        "turn.Jsonl.zstd",
        "turn.jsonl.ZSTD",
        "turn.jsonlines",
        "turn.v3.NDJSON.zstd",
        ".jsonl",
        "settings.json",
        ".claude.json",
        "jsonl",
    ]
    .map(|name| packing(Path::new("/h").join(name).as_path()));
    let unread = Some(Packing::Unread);
    assert_eq!(
        named,
        [
            unread, unread, unread, unread, unread, unread, unread, unread, unread, None, None,
            None
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
