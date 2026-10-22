//! What `inspect --capability-call` writes of a verified response on a
//! terminal and off one (decision 0065 slice two, U8g). The binary's
//! output is never a terminal in the CLI suite, which proves the bytes
//! off one; this proves the terminal's line.

use super::shown;

/// Off a terminal, a response is written exactly, a byte that is not
/// text and an escape included; on one, its text loses every control and
/// reordering character, C1 and bidirectional marks included, and its
/// line is ended.
#[test]
fn a_response_is_exact_off_a_terminal_and_cannot_steer_one() {
    let bytes = "{\"t\":\"a\u{1b}[2Jb\u{9b}c\u{202e}d\u{2066}e\"}\n".as_bytes();
    let bytes = [bytes, &[0xff]].concat();
    assert_eq!(&*shown(&bytes, false), &bytes[..]);
    let painted = "{\"t\":\"a[2Jbcde\"}\u{fffd}\n";
    assert_eq!(&*shown(&bytes, true), painted.as_bytes());
}
