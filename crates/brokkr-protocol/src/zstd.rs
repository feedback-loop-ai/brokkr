//! The realm's one zstd decoder (#484, the operator's ruling of
//! 2026-10-04; decision 0071 ruling 5): concatenated frames decoded in
//! turn by the pure-Rust, decode-only `ruzstd`, each trusted only once its
//! content checksum vouches for it, the whole under a bound. The hands
//! probe reads a harness's compressed transcript through it, and the local
//! transcript reader a compressed DSH session (proposed decision 0086).

use std::io::Read;

use ruzstd::decoding::{FrameDecoder, StreamingDecoder};

/// What a bounded decompression established.
#[derive(Debug, PartialEq, Eq)]
pub struct Unpacked {
    /// The decompressed bytes, never more than the bound.
    pub bytes: Vec<u8>,
    /// The source holds more than `bytes`: what it decompresses to passed
    /// the bound, or the caller's own read of it was cut.
    pub past_bound: bool,
}

/// Why a compressed source is refused.
#[derive(Debug, thiserror::Error)]
pub enum Refused {
    /// The decoder could not read a frame whole: bytes that are no zstd
    /// frame, a frame cut short, or a skippable frame, which is never read
    /// as nothing. The decoder's own words.
    #[error("{0}")]
    Undecodable(std::io::Error),
    /// A frame read whole that states no content checksum, so nothing can
    /// vouch for its bytes.
    #[error("a frame carries no content checksum to verify it by")]
    Unchecksummed,
    /// A frame read whole whose bytes do not match the checksum it states.
    #[error("a frame's bytes do not match its content checksum {0:#010x}")]
    Mismatch(u32),
}

/// `packed`'s frames decompressed in turn, at most `bound` bytes of them.
///
/// `cut` says `packed` is only a prefix of its source, the caller's read
/// having stopped at a bound of its own: input that ends inside a frame
/// is then the edge of that read, not a frame cut short, so the frame
/// contributes nothing and the outcome is past the bound. A frame read to
/// its end is kept only when it states a content checksum and its bytes
/// match it. The one frame the output bound stops is not read to its end,
/// so its checksum is never reached; nothing past the bound is decoded.
pub fn unpack(packed: &[u8], cut: bool, bound: u64) -> Result<Unpacked, Refused> {
    let mut source = Edge {
        rest: packed,
        reached: false,
    };
    let mut bytes = Vec::new();
    while !source.rest.is_empty() {
        let start = bytes.len();
        match frame_into(&mut source, &mut bytes, bound) {
            Ok(Frame::Whole) => {}
            Ok(Frame::PastBound) => {
                bytes.truncate(bound as usize);
                return Ok(Unpacked {
                    bytes,
                    past_bound: true,
                });
            }
            Err(_) if cut && source.reached => {
                bytes.truncate(start);
                return Ok(Unpacked {
                    bytes,
                    past_bound: true,
                });
            }
            Err(refused) => return Err(refused),
        }
    }
    Ok(Unpacked {
        bytes,
        past_bound: cut,
    })
}

/// How one frame ended.
enum Frame {
    /// Read to its end, its checksum verified.
    Whole,
    /// Stopped one byte past the bound.
    PastBound,
}

/// Decode the next frame of `source` onto `bytes`, at most one byte past
/// `bound` in all.
fn frame_into(source: &mut Edge<'_>, bytes: &mut Vec<u8>, bound: u64) -> Result<Frame, Refused> {
    let mut frame = StreamingDecoder::new(&mut *source)
        .map_err(|error| Refused::Undecodable(std::io::Error::other(error)))?;
    // Within the bound so far, so one byte past what it leaves.
    let room = bound - bytes.len() as u64 + 1;
    (&mut frame)
        .take(room)
        .read_to_end(bytes)
        .map_err(Refused::Undecodable)?;
    if bytes.len() as u64 > bound {
        return Ok(Frame::PastBound);
    }
    checksummed(&frame.into_frame_decoder())?;
    Ok(Frame::Whole)
}

/// Whether `frame`, read to its end, carries the content checksum of the
/// bytes it decompressed to. The streaming decoder compares nothing, and
/// most single-bit corruption decodes to changed bytes without an error,
/// so a frame is trusted only when its checksum is stated and matches,
/// and one stating none cannot be verified and is refused too (#484, the
/// operator's ruling of 2026-10-04).
fn checksummed(frame: &FrameDecoder) -> Result<(), Refused> {
    match frame.get_checksum_from_data() {
        None => Err(Refused::Unchecksummed),
        Some(stated) if frame.get_calculated_checksum() == Some(stated) => Ok(()),
        Some(stated) => Err(Refused::Mismatch(stated)),
    }
}

/// The caller's input, noting whether the decoder asked past its end.
struct Edge<'a> {
    rest: &'a [u8],
    reached: bool,
}

impl Read for Edge<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if self.rest.is_empty() && !buffer.is_empty() {
            self.reached = true;
        }
        self.rest.read(buffer)
    }
}

#[cfg(test)]
mod tests;
