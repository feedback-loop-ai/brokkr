//! DSH 0.2's session files through the shared reader (change
//! `admit-dsh-session-v4`, proposed decision 0086): both version-four
//! names found and ordered, a compressed candidate's header and body read
//! decompressed within the bounds a plain one has, and a frame that fails
//! refused at the stage that read it.

use std::path::{Path, PathBuf};

use brokkr_view::transcript::{TranscriptRead, Unavailable};

use super::tests::{common, read_common};

/// The synthetic version-four session and its compressed twin, one
/// checksummed frame per line as DSH's backend appends them.
const PLAIN: &[u8] = include_bytes!("../../tests/fixtures/dsh-v4/session.v4.jsonl");
const PACKED: &[u8] = include_bytes!("../../tests/fixtures/dsh-v4/session.v4.jsonl.zstd");
/// One-frame headers of exactly the header cap and one byte past it.
const AT_CAP: &[u8] = include_bytes!("../../tests/fixtures/dsh-v4/header-at-cap.v4.jsonl.zstd");
const PAST_CAP: &[u8] = include_bytes!("../../tests/fixtures/dsh-v4/header-past-cap.v4.jsonl.zstd");
/// A header, a user message and a row that decompresses past the source
/// cap, a frame each.
const PAST_SOURCE: &[u8] =
    include_bytes!("../../tests/fixtures/dsh-v4/past-source-cap.v4.jsonl.zstd");

/// A home holding one DSH session directory under `sessions/<name>`.
struct Home {
    _dir: tempfile::TempDir,
    home: PathBuf,
}

impl Home {
    fn new() -> Home {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        Home { _dir: dir, home }
    }

    /// Write `files` into the session directory of `name`.
    fn session(&self, name: &str, files: &[(&str, &[u8])]) -> PathBuf {
        let session = self.home.join("sessions").join(name).join("project/seat");
        std::fs::create_dir_all(&session).unwrap();
        for (file, bytes) in files {
            std::fs::write(session.join(file), bytes).unwrap();
        }
        session
    }

    fn read(&self, name: &str) -> TranscriptRead {
        let locator = format!("sessions/{name}");
        read_common(&common(
            "dsh-session",
            &locator,
            self.home.to_str().unwrap(),
        ))
    }
}

/// What a read shows: each turn's role, stamp and block texts.
fn shown(read: &TranscriptRead) -> Vec<(String, String, Vec<String>)> {
    read.turns
        .iter()
        .map(|turn| {
            let texts = turn.blocks.iter().map(|block| block.text.clone()).collect();
            (turn.role.clone(), turn.ts.clone(), texts)
        })
        .collect()
}

fn ends_with(read: &TranscriptRead, name: &str) -> bool {
    read.path
        .as_deref()
        .is_some_and(|path| Path::new(path).ends_with(name))
}

/// The committed twin is the plain fixture, frame for frame.
#[test]
fn the_compressed_fixture_is_the_plain_one() {
    let unpacked = brokkr_protocol::zstd::unpack(PACKED, false, 1 << 20).unwrap();
    assert_eq!(
        (unpacked.bytes.as_slice(), unpacked.past_bound),
        (PLAIN, false)
    );
}

/// `session.v4.jsonl` and `session.v4.jsonl.zstd` each read into the same
/// five turns, nothing counted and nothing truncated.
#[test]
fn both_version_four_names_read_into_the_same_turns() {
    let home = Home::new();
    let turn = |role: &str, ts: &str, text: &str| (role.into(), ts.into(), vec![text.into()]);
    let expected = vec![
        turn("user", "1760400000108", "Read the synthetic file."),
        turn("assistant", "1760400000109", "Reading it."),
        turn(
            "assistant",
            "1760400000110",
            "read {\"path\":\"a.txt\"} [call-1]",
        ),
        turn("tool", "1760400000111", "synthetic alpha [call-1]"),
        turn(
            "assistant",
            "1760400000116",
            "The file says synthetic alpha.",
        ),
    ];
    for (name, bytes) in [
        ("session.v4.jsonl", PLAIN),
        ("session.v4.jsonl.zstd", PACKED),
    ] {
        home.session(name, &[(name, bytes)]);
        let read = home.read(name);
        assert!(ends_with(&read, name), "{read:?}");
        assert_eq!(shown(&read), expected, "{name}");
        assert_eq!(
            (
                read.unavailable,
                read.truncated,
                read.skipped_lines,
                read.unrecognized_records
            ),
            (None, false, 0, 0),
            "{name}"
        );
        assert!(read.notices.is_empty(), "{name}: {:?}", read.notices);
    }
}

/// One directory holding every admitted name admits the newest: version
/// four plain, then version four compressed, then version three, then the
/// plain name, each read only once every newer one is gone.
#[test]
fn the_admitted_names_are_asked_newest_first() {
    let home = Home::new();
    let says = |text: &str| {
        format!(
            "{{\"type\":\"session\",\"version\":0,\"delegationDepth\":0}}\n\
             {{\"type\":\"user/message\",\"seq\":1,\"time\":1,\"data\":{{\"content\":\"{text}\"}}}}\n"
        )
    };
    let (four, three, zero) = (says("v4 plain"), says("v3"), says("plain"));
    let names = [
        ("session.v4.jsonl", four.as_bytes()),
        ("session.v4.jsonl.zstd", PACKED),
        ("session.v3.jsonl", three.as_bytes()),
        ("session.jsonl", zero.as_bytes()),
    ];
    let session = home.session("one", &names);
    let mut seen = Vec::new();
    for (name, _) in names {
        let read = home.read("one");
        assert!(ends_with(&read, name), "{read:?}");
        seen.push(read.turns[0].blocks[0].text.clone());
        std::fs::remove_file(session.join(name)).unwrap();
    }
    assert_eq!(
        seen,
        ["v4 plain", "Read the synthetic file.", "v3", "plain"]
    );
}

/// A compressed header's budget is its decompressed bytes: a header of
/// exactly the cap is admitted, one byte more is `discovery-limit`, and so
/// is a header the decoder has not reached within twice the cap of
/// compressed input.
#[test]
fn a_compressed_header_is_bounded_by_what_it_decompresses_to() {
    // The fixtures are the boundary: a header of exactly the cap, and one
    // byte more, each up to its newline once decompressed.
    let cap = brokkr_view::transcript::DSH_HEADER_CAP;
    for (packed, header) in [(AT_CAP, cap), (PAST_CAP, cap + 1)] {
        let plain = brokkr_protocol::zstd::unpack(packed, false, 1 << 20).unwrap();
        assert!(!plain.past_bound);
        assert_eq!(
            plain.bytes.iter().position(|byte| *byte == b'\n'),
            Some(header)
        );
    }
    let home = Home::new();
    let name = "session.v4.jsonl.zstd";
    home.session("at", &[(name, AT_CAP)]);
    home.session("past", &[(name, PAST_CAP)]);
    home.session("late", &[(name, &late_header())]);
    let at = home.read("at");
    assert_eq!((at.unavailable, at.turns.len()), (None, 0), "{at:?}");
    for name in ["past", "late"] {
        let read = home.read(name);
        assert_eq!(
            read.unavailable,
            Some(Unavailable::DiscoveryLimit),
            "{name}"
        );
        assert_eq!(read.path, None, "{name}");
    }
}

/// A frame stating no checksum whose blocks carry no bytes until past
/// twice the header cap of compressed input, then a session header.
fn late_header() -> Vec<u8> {
    // Magic, a descriptor with a one-byte window field and no checksum,
    // a 1 MiB window, then empty raw blocks and one raw last block.
    let mut frame = vec![0x28, 0xb5, 0x2f, 0xfd, 0x00, 0x50];
    let empty = 2 * brokkr_view::transcript::DSH_HEADER_CAP / 3 + 1;
    for _ in 0..empty {
        frame.extend([0, 0, 0]);
    }
    let header = b"{\"type\":\"session\",\"version\":4,\"delegationDepth\":0}\n";
    let size = (header.len() << 3) | 1;
    frame.extend(&size.to_le_bytes()[..3]);
    frame.extend(header);
    frame
}

/// A compressed candidate whose frame states no checksum, fails it, or is
/// no zstd at all is `unreadable` at discovery, decisively: the
/// version-three file beside it is never read.
#[test]
fn a_compressed_candidate_whose_frame_fails_is_unreadable() {
    let home = Home::new();
    let mut flipped = PACKED.to_vec();
    let last = flipped.len() - 1;
    flipped[last] ^= 1;
    let header_frame = frame_end(1);
    let mut bare = PACKED[..header_frame - 4].to_vec();
    bare[4] &= !0b100;
    bare.extend(&PACKED[header_frame..]);
    let beside = b"{\"type\":\"session\",\"version\":3,\"delegationDepth\":0}\n";
    for (case, bytes) in [
        ("flipped", &flipped[..]),
        ("bare", &bare[..]),
        ("plain", PLAIN),
    ] {
        home.session(
            case,
            &[
                ("session.v4.jsonl.zstd", bytes),
                ("session.v3.jsonl", beside),
            ],
        );
        let read = home.read(case);
        assert_eq!(
            (read.unavailable, read.path.clone()),
            (Some(Unavailable::Unreadable), None),
            "{case}: {read:?}"
        );
    }
}

/// Where the twin's frame for line `lines` ends, one frame holding each
/// line: the shortest prefix that decompresses to the first `lines` lines.
fn frame_end(lines: usize) -> usize {
    let plain = PLAIN
        .iter()
        .enumerate()
        .filter(|(_, byte)| **byte == b'\n')
        .nth(lines - 1)
        .map(|(at, _)| at + 1)
        .unwrap();
    (1..=PACKED.len())
        .find(|end| {
            brokkr_protocol::zstd::unpack(&PACKED[..*end], false, 1 << 20)
                .is_ok_and(|unpacked| unpacked.bytes == PLAIN[..plain])
        })
        .unwrap()
}

/// A compressed body whose compressed bytes pass the source cap before
/// what they decompress to does is read to the cap's last whole frame and
/// truncated: the frame the cap cuts contributes nothing and is no frame
/// cut short.
#[test]
fn a_compressed_body_past_the_input_cap_truncates_at_its_last_whole_frame() {
    let (header, turn_start) = (frame_end(1), frame_end(2));
    let cap = brokkr_view::transcript::SOURCE_CAP as usize;
    let quiet = &PACKED[header..turn_start];
    let mut packed = PACKED[..header].to_vec();
    while packed.len() <= cap {
        packed.extend(quiet);
    }
    let home = Home::new();
    home.session("one", &[("session.v4.jsonl.zstd", &packed)]);
    let read = home.read("one");
    assert_eq!(
        (
            read.unavailable,
            read.turns.len(),
            read.truncated,
            read.skipped_lines,
            read.unrecognized_records
        ),
        (None, 0, true, 0, 0),
        "{:?}",
        read.notices
    );
}

/// Only the two version-four names are admitted: no other compressed
/// spelling, and no compressed twin of an older name, is a candidate.
#[test]
fn no_other_compressed_name_is_read() {
    let home = Home::new();
    for name in [
        "session.v4.jsonl.zst",
        "session.v3.jsonl.zstd",
        "session.jsonl.zstd",
        "session.v4.jsonl.ZSTD",
        "session.v04.jsonl.zstd",
    ] {
        let session = home.session(name, &[(name, PACKED)]);
        // A filesystem that folds case, macOS's by default, holds an
        // upper-case spelling as the admitted name itself.
        if name != name.to_lowercase() && session.join(name.to_lowercase()).exists() {
            continue;
        }
        assert_eq!(
            home.read(name).unavailable,
            Some(Unavailable::NotFound),
            "{name}"
        );
    }
}

/// A compressed body that decompresses past the source cap reads exactly
/// as the same bytes written plain: the complete rows before the cap, the
/// source truncated, nothing counted.
#[test]
fn a_compressed_body_past_the_source_cap_truncates_as_a_plain_one() {
    let head = "{\"type\":\"session\",\"version\":4,\"delegationDepth\":0,\"isSeeded\":false}\n";
    let small = "{\"type\":\"user/message\",\"seq\":0,\"time\":1000,\"data\":{\"content\":[{\"type\":\"text\",\"text\":\"q\"}]}}\n";
    let big = format!(
        "{{\"type\":\"user/message\",\"seq\":1,\"time\":1001,\"data\":{{\"content\":[{{\"type\":\"text\",\"text\":\"{}\"}}]}}}}\n",
        "x".repeat(33_554_432)
    );
    let plain = [head.as_bytes(), small.as_bytes(), big.as_bytes()].concat();
    let whole = brokkr_protocol::zstd::unpack(PAST_SOURCE, false, 1 << 26).unwrap();
    assert_eq!(whole.bytes, plain, "the committed fixture is these bytes");

    let home = Home::new();
    home.session("plain", &[("session.v4.jsonl", &plain)]);
    home.session("packed", &[("session.v4.jsonl.zstd", PAST_SOURCE)]);
    let (plain, packed) = (home.read("plain"), home.read("packed"));
    assert_eq!(
        shown(&packed),
        [("user".into(), "1000".into(), vec!["q".into()])]
    );
    assert_eq!(
        (
            packed.unavailable,
            packed.truncated,
            packed.skipped_lines,
            packed.unrecognized_records
        ),
        (None, true, 0, 0)
    );
    assert_eq!(
        (shown(&packed), packed.truncated, &packed.notices),
        (shown(&plain), plain.truncated, &plain.notices)
    );
}

/// A frame past what discovery decodes that fails its checksum refuses
/// the body read: `unreadable`, keeping the confirmed path.
#[test]
fn a_body_frame_that_fails_is_unreadable_with_its_path() {
    let mut packed = [PACKED, PAST_CAP, PACKED].concat();
    let last = packed.len() - 1;
    packed[last] ^= 1;
    let home = Home::new();
    let session = home.session("one", &[("session.v4.jsonl.zstd", &packed)]);
    let read = home.read("one");
    let path = session
        .canonicalize()
        .unwrap()
        .join("session.v4.jsonl.zstd");
    assert_eq!(
        (read.unavailable, read.path.as_deref(), read.turns.len()),
        (Some(Unavailable::Unreadable), path.to_str(), 0),
        "{read:?}"
    );
}
