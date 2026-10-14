//! The bounded decoder's own outcomes, beside the probe's refusals of the
//! same frames (`probe/tests/compressed.rs`): a read the bound stops keeps
//! its bytes and says there is more, and a read the caller cut reads its
//! last frame as the edge of that read, never as a frame cut short.

use super::{unpack, Refused, Unpacked};

/// The controller's dsh skeleton and its zstd CLI 1.5.7 twin, one
/// checksummed frame (the probe's fixture).
const SKELETON: &[u8] = include_bytes!("../probe/measure/streams/dsh-plain.jsonl");
const PACKED: &[u8] = include_bytes!("../probe/measure/streams/dsh-plain.jsonl.zstd");

/// A bound inside a frame keeps exactly the bound's bytes and says the
/// source holds more; a bound at a frame's end keeps every byte and says
/// nothing more.
#[test]
fn the_bound_keeps_its_bytes_and_says_there_is_more() {
    let len = SKELETON.len();
    let two = [PACKED, PACKED].concat();
    let both = [SKELETON, SKELETON].concat();
    let read = |packed: &[u8], bound: usize| unpack(packed, false, bound as u64).unwrap();
    assert_eq!(
        [read(PACKED, 10), read(&two, len + 1), read(&two, 2 * len)],
        [
            Unpacked {
                bytes: SKELETON[..10].to_vec(),
                past_bound: true
            },
            Unpacked {
                bytes: both[..len + 1].to_vec(),
                past_bound: true
            },
            Unpacked {
                bytes: both,
                past_bound: false
            },
        ]
    );
}

/// Input the caller cut ends its read: a frame it ends inside contributes
/// nothing, one it ends after is kept, and either way the source holds
/// more. The same input not cut is a frame cut short, refused.
#[test]
fn a_cut_read_ends_at_its_last_whole_frame() {
    let two = [PACKED, PACKED].concat();
    let inside = &two[..PACKED.len() + 100];
    let read = |packed: &[u8]| unpack(packed, true, 1 << 20).unwrap();
    assert_eq!(
        [read(inside), read(PACKED), read(&PACKED[..100])],
        [
            Unpacked {
                bytes: SKELETON.to_vec(),
                past_bound: true
            },
            Unpacked {
                bytes: SKELETON.to_vec(),
                past_bound: true
            },
            Unpacked {
                bytes: Vec::new(),
                past_bound: true
            },
        ]
    );
    let refused = unpack(inside, false, 1 << 20).unwrap_err();
    assert!(matches!(refused, Refused::Undecodable(_)), "{refused:?}");
}

/// A cut read still refuses a frame read whole that its checksum does not
/// vouch for, and bytes that are no frame at all: only running out of
/// the caller's input is the edge of the read.
#[test]
fn a_cut_read_still_refuses_what_it_read_whole() {
    let mut corrupt = PACKED.to_vec();
    let last = corrupt.len() - 1;
    corrupt[last] ^= 1;
    let mut bare = PACKED[..PACKED.len() - 4].to_vec();
    bare[4] &= !0b100;
    let outcome = |packed: &[u8]| match unpack(packed, true, 1 << 20) {
        Err(Refused::Mismatch(_)) => "mismatch",
        Err(Refused::Unchecksummed) => "unchecksummed",
        Err(Refused::Undecodable(_)) => "undecodable",
        Ok(_) => "read",
    };
    assert_eq!(
        [
            outcome(&[&corrupt[..], PACKED].concat()),
            outcome(&[&bare[..], PACKED].concat()),
            outcome(SKELETON)
        ],
        ["mismatch", "unchecksummed", "undecodable"]
    );
}
