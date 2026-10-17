//! The broker's own standard error (decision 0065 slice two, U6f1b; MB3,
//! MB4): one voice every line passes, sanitized before it is masked and
//! guarded, and cut at its safe points into pieces one write takes, each
//! safe on its own; the last line rendered whole, or none where its
//! bindings are unknown; and one writer, on its caller's thread, that
//! never waits past its bound or a cancellation, ends at a write taken
//! short, and has returned from every write when it returns.

use std::io::{ErrorKind, PipeReader, Read, Write};
use std::os::fd::{AsFd, BorrowedFd};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use brokkr_protocol::broker::ledger::Failure;
use brokkr_protocol::secret::Guard;

use super::{last, told_to, Drain, Said, Voice, SERVER, TOLD};
use crate::broker::rpc::pieces::tests::{bound, held, sink, within};
use crate::broker::rpc::pieces::Put;
use crate::broker::rpc::{Fatal, Line, Lines, ATOMIC};
use crate::broker::session::{Ended, Startup};

/// What `said` holds, as text.
fn text(said: Option<Said>) -> Option<String> {
    said.map(|said| said.as_str().to_owned())
}

#[test]
fn a_line_is_sanitized_for_display_before_it_is_masked_and_guarded() {
    // ESC, OSC 52's BEL and a bare CR are dropped as every rendered line
    // drops them; a bound value split by a control byte is joined by that
    // and masked whole, where masking the raw bytes alone would miss it.
    let bindings = bound("DOCS_TOKEN", "u6c8-canary-Qx7Zv");
    let voice = Voice::new(&bindings);
    let sequences = b"clear \x1b[2J copy \x1b]52;c;aGk=\x07 back\rover";
    let shown = "clear [2J copy ]52;c;aGk= backover\n";
    assert_eq!(text(voice.said(sequences, "fixed")), Some(shown.to_owned()));
    let split = b"split u6c8-ca\x01nary-Qx7Zv end";
    let masked = "split [secret:DOCS_TOKEN] end\n";
    assert_eq!(text(voice.said(split, "fixed")), Some(masked.to_owned()));
    // An empty line is kept as its newline.
    assert_eq!(text(voice.said(b"", "fixed")), Some("\n".to_owned()));
}

#[test]
fn the_last_line_is_rendered_whole_and_written_only_by_known_bindings() {
    let protocol = || Err(Ended::Failed(Fatal::Failed(Failure::Protocol)));
    let error = bound("DOCS_TOKEN", "error");
    let masked = "[secret:DOCS_TOKEN]: MCP server protocol is invalid\n";
    let (code, said) = last(protocol(), Some(&error));
    assert_eq!(
        (code, text(said)),
        (ExitCode::from(1), Some(masked.to_owned()))
    );
    // None looked up: the line as any failed command prints it.
    let plain = "error: MCP server protocol is invalid\n";
    let (code, said) = last(protocol(), Some(&[]));
    assert_eq!(
        (code, text(said)),
        (ExitCode::from(1), Some(plain.to_owned()))
    );
    // Bindings unknown: no line, and the exit code alone.
    let (code, said) = last(protocol(), None);
    assert_eq!((code, text(said)), (ExitCode::from(1), None));
    let (code, said) = last(Ok(()), Some(&error));
    assert_eq!((code, text(said)), (ExitCode::SUCCESS, None));
}

#[test]
fn every_piece_of_a_servers_line_is_labelled_within_its_bound_and_the_brokers_own_never() {
    // The label's one pin (U6f2; operator ruling 2026-10-14).
    assert_eq!(SERVER, "MCP server stderr: ");
    let bindings = bound("DOCS_TOKEN", "u6c8-canary-Qx7Zv");
    let (reader, mut writer) = std::io::pipe().unwrap();
    // A server's `error: `, and a line two pieces long: each piece begins
    // with the label, which counts inside its 512 bytes.
    let long = "y".repeat(2 * (ATOMIC - 1 - SERVER.len()) - 1) + "z";
    writeln!(writer, "error: boom\n{long}").unwrap();
    drop(writer);
    let tail: Vec<String> = Drain::new(reader, &bindings)
        .tail()
        .iter()
        .flat_map(|said| said.0.split_inclusive('\n').map(str::to_owned))
        .collect();
    let half = "y".repeat(ATOMIC - 1 - SERVER.len());
    let rest = "y".repeat(ATOMIC - 2 - SERVER.len()) + "z";
    let pieces = ["error: boom", &half, &rest].map(|piece| format!("{SERVER}{piece}\n"));
    assert_eq!(tail, pieces);
    assert_eq!(tail.iter().map(String::len).max(), Some(ATOMIC));
    // A line no labelled piece can cut safely is refused for the labelled
    // marker; the broker's own line is never labelled.
    let digits = "1".repeat(ATOMIC - SERVER.len());
    let server = Voice {
        label: SERVER,
        ..Voice::new(&bindings)
    };
    let marker = format!("{SERVER}fixed\n");
    assert_eq!(text(server.said(digits.as_bytes(), "fixed")), Some(marker));
    let own = Voice::new(&bindings).said(b"error: boom", "");
    assert_eq!(text(own), Some("error: boom\n".to_owned()));
}

#[test]
fn a_sink_that_takes_nothing_is_given_up_on_at_its_bound_and_one_that_reads_takes_it_all() {
    let bindings = bound("DOCS_TOKEN", "u6c8-canary-Qx7Zv");
    let voice = Voice::new(&bindings);
    let lines = || {
        vec![
            voice.said(b"one", "").unwrap(),
            voice.said(b"two", "").unwrap(),
        ]
    };
    // A pipe whose room is filled and that nobody reads.
    let (reader, mut writer) = std::io::pipe().unwrap();
    rustix::io::ioctl_fionbio(&writer, true).unwrap();
    while writer.write(&[b'x'; 4096]).is_ok() {}
    while writer.write(b"x").is_ok() {}
    rustix::io::ioctl_fionbio(&writer, false).unwrap();
    let mode = rustix::fs::fcntl_getfl(&writer).unwrap();
    // Five seconds, so a broker descheduled under load still says its
    // last line (the operator's ruling of 2026-10-17, U6f3).
    assert_eq!(TOLD, Duration::from_secs(5));
    let begun = Instant::now();
    let whole = told_to(&lines(), writer.try_clone().unwrap(), within(TOLD));
    let taken = begun.elapsed();
    let bounded = (taken >= TOLD) & (taken < TOLD + Duration::from_millis(500));
    assert_eq!((whole, bounded), (Put::Late, true), "{taken:?}");
    assert_eq!(rustix::fs::fcntl_getfl(&writer).unwrap(), mode);
    // Cancelled as it waits: given up on at once, not at its bound.
    let cancelled: Startup<'_> = (Instant::now() + TOLD, &|| true);
    let begun = Instant::now();
    let whole = told_to(&lines(), writer.try_clone().unwrap(), cancelled);
    let taken = begun.elapsed();
    assert_eq!((whole, taken < TOLD / 2), (Put::Late, true), "{taken:?}");
    drop(reader);
    // A pipe its reader reads: every line, in order.
    let (reader, writer) = std::io::pipe().unwrap();
    assert_eq!(told_to(&lines(), writer, within(TOLD)), Put::Taken);
    assert_eq!(held(reader), b"one\ntwo\n");
}

#[test]
fn a_line_one_write_cannot_take_is_cut_only_at_its_safe_points() {
    // With 0012 bound, a line ending in 123, written plain or as the
    // escapes the guard decodes, passes whole, as 123; one write of 512
    // bytes would end in 12 or its escapes, the bound value. It is cut
    // before its last x instead, so a sink that takes one write and
    // refuses the next holds a piece the guard passes, and nothing of the
    // number (U6f1b; MB4; operator ruling 2026-10-13).
    let bindings = bound("DOCS_TOKEN", "0012");
    let voice = Voice::new(&bindings);
    let guard = Guard::new(&bindings);
    let escaped = concat!("\\", "u0031", "\\", "u0032", "\\", "u0033");
    for (number, xs) in [("123", ATOMIC - 3), (escaped, ATOMIC - 13)] {
        let straddling = ["x".repeat(xs), number.to_string()].concat();
        let whole = guard.text(straddling.as_bytes());
        let one_write = guard.text(&straddling.as_bytes()[..ATOMIC - 1]);
        assert_eq!((number, whole, one_write), (number, true, false));
        let said = [
            voice.said(straddling.as_bytes(), "fixed").unwrap(),
            voice.said(b"next", "").unwrap(),
        ];
        let first = "x".repeat(xs - 1) + "\n";
        let pieces = [first.clone(), format!("x{number}\n"), "next\n".to_string()];
        let cut: Vec<&str> = said
            .iter()
            .flat_map(|said| said.0.split_inclusive('\n'))
            .collect();
        assert_eq!(
            (number, cut),
            (number, pieces.iter().map(String::as_str).collect())
        );
        let (one, reader) = sink(ATOMIC, 1, Duration::ZERO);
        let whole = told_to(&said, one, within(TOLD));
        let holds = String::from_utf8(held(reader)).unwrap();
        let passed = guard.text(holds.as_bytes());
        let refused = (number, first, Put::Closed, true);
        assert_eq!((number, holds, whole, passed), refused);
    }
    // A line whose every 511 bytes hold no safe point cannot be cut
    // safely: its fixed line is said in its place.
    let digits = "1".repeat(ATOMIC);
    assert_eq!(
        text(voice.said(digits.as_bytes(), "fixed")),
        Some("fixed\n".into())
    );
    // A line whose every piece passes is cut into them, each a write.
    let long = ["y".repeat(2 * ATOMIC - 1), "z".to_string()].concat();
    let pieces = [
        "y".repeat(ATOMIC - 1),
        "y".repeat(ATOMIC - 1),
        "yz".to_string(),
    ];
    let expected = pieces.map(|piece| piece + "\n").concat();
    assert_eq!(text(voice.said(long.as_bytes(), "")), Some(expected));
}

#[test]
fn a_write_taken_short_ends_the_writing_whatever_it_took() {
    // A sink that takes three bytes a write, as a terminal or a file may
    // take less than it is offered: the first such write ends the
    // writing, taken short, and nothing more is offered; what it took is
    // no emission of the broker's (operator ruling 2026-10-13).
    let bindings = bound("DOCS_TOKEN", "u6c8-canary-Qx7Zv");
    let voice = Voice::new(&bindings);
    let said = [
        voice.said(b"abcde", "").unwrap(),
        voice.said(b"next", "").unwrap(),
    ];
    let (short, reader) = sink(3, usize::MAX, Duration::ZERO);
    let at = std::sync::Arc::clone(&short.at);
    let whole = told_to(&said, short, within(TOLD));
    let writes = at.lock().unwrap().len();
    assert_eq!(
        (held(reader), whole, writes),
        (b"abc".to_vec(), Put::Short, 1)
    );
}

#[test]
fn a_cancellation_or_a_deadline_between_pieces_writes_nothing_more() {
    // Cancelled once the first piece is taken: the second is never
    // offered. A deadline the first write outlives: likewise.
    let bindings = bound("DOCS_TOKEN", "u6c8-canary-Qx7Zv");
    let voice = Voice::new(&bindings);
    let said = || {
        [
            voice.said(b"one", "").unwrap(),
            voice.said(b"two", "").unwrap(),
        ]
    };
    let (taking, reader) = sink(ATOMIC, usize::MAX, Duration::ZERO);
    let at = std::sync::Arc::clone(&taking.at);
    let cancelled = move || !at.lock().unwrap().is_empty();
    let attempt: Startup<'_> = (Instant::now() + TOLD, &cancelled);
    let whole = told_to(&said(), taking, attempt);
    assert_eq!((held(reader), whole), (b"one\n".to_vec(), Put::Late));
    let (slow, reader) = sink(ATOMIC, usize::MAX, Duration::from_millis(300));
    let whole = told_to(&said(), slow, within(Duration::from_millis(100)));
    assert_eq!((held(reader), whole), (b"one\n".to_vec(), Put::Late));
}

#[test]
fn every_write_has_returned_before_the_writer_does() {
    // Each write takes a while: every one has returned before the writer
    // does, so none outlives the bindings it was guarded by, which `serve`
    // drops only then (U6f1b; MB4).
    let bindings = bound("DOCS_TOKEN", "u6c8-canary-Qx7Zv");
    let voice = Voice::new(&bindings);
    let said = [
        voice.said(b"one", "").unwrap(),
        voice.said(b"two", "").unwrap(),
        voice.said(b"three", "").unwrap(),
    ];
    let (sink, reader) = sink(ATOMIC, usize::MAX, Duration::from_millis(100));
    let at = std::sync::Arc::clone(&sink.at);
    let whole = told_to(&said, sink, within(TOLD));
    let returned = Instant::now();
    let at = at.lock().unwrap().clone();
    let before = at.iter().all(|at| *at <= returned);
    assert_eq!((whole, at.len(), before), (Put::Taken, 3, true));
    assert_eq!(held(reader), b"one\ntwo\nthree\n");
}

#[test]
fn a_piece_the_sink_was_not_ready_for_is_offered_again() {
    // The broker's standard error refuses the first write as not ready, as
    // a status flag another holder set would: the piece is offered again,
    // and taken whole.
    let said = [Voice::new(&[]).said(b"one", "").unwrap()];
    let (mut unready, reader) = sink(ATOMIC, usize::MAX, Duration::ZERO);
    unready.unready = 1;
    let at = std::sync::Arc::clone(&unready.at);
    let whole = told_to(&said, unready, within(TOLD));
    let writes = at.lock().unwrap().len();
    assert_eq!(
        (whole, writes, held(reader)),
        (Put::Taken, 1, b"one\n".to_vec())
    );
}

/// A pipe holding a byte, so poll finds it readable, whose every read errs
/// with its kind.
struct Erring(PipeReader, ErrorKind);

impl Read for Erring {
    fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
        Err(self.1.into())
    }
}

impl AsFd for Erring {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

#[test]
fn a_read_a_signal_interrupts_is_quiet_and_any_other_read_error_ends_the_pipe() {
    // The server's standard error, like the harness's input, is read as
    // lines: a read a signal interrupted took nothing yet, and any other
    // error is the pipe's end.
    for (kind, line) in [
        (ErrorKind::Interrupted, Line::Quiet),
        (ErrorKind::Other, Line::Ended),
    ] {
        let (reader, mut writer) = std::io::pipe().unwrap();
        writer.write_all(b"x").unwrap();
        let mut lines = Lines::new(Erring(reader, kind));
        assert_eq!((kind, lines.next(8)), (kind, line));
    }
}
