//! What the loopback UI holds of a client (#433, decision 0079's proposed
//! addendum). A stuck or runaway local client, a hung tab, a script that
//! never closes, a request with no end of headers, must not exhaust the
//! surface's threads or memory.
//!
//! A request line is refused past [`REQUEST_LINE_BYTES`] and a header
//! section past [`HEADER_BYTES`], at the first byte past the limit and
//! before the line is held whole. The request must arrive within
//! [`READ_TIMEOUT`] of its connection, and each write waits at most
//! [`WRITE_TIMEOUT`] for a client to read. At most [`CONNECTIONS`]
//! connections are served at once: a slot is taken before a thread is
//! started and given back when the thread ends, and a connection past the
//! cap is answered at once from the acceptor and closed, never queued and
//! never given a thread. A server-sent stream holds its slot for as long as
//! it is open.
//!
//! Every limit is compared without overflow, so no value of one means
//! unlimited, and zero admits nothing: a zero write timeout is refused by
//! the platform, so that connection closes unanswered. A refusal answers
//! with the status HTTP gives it and a fixed body naming the limit and its
//! unit, never the client's bytes.

use std::fmt;
use std::io::{self, BufRead, ErrorKind, Read};
use std::net::TcpStream;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::json;

use super::Response;

/// The most bytes of the request line, its line ending included: 8 KiB.
pub(super) const REQUEST_LINE_BYTES: usize = 8 * 1024;

/// The most bytes of the header section, every header line and the blank
/// line that ends them included: 32 KiB.
pub(super) const HEADER_BYTES: usize = 32 * 1024;

/// The most time from a connection's accept to the end of its request's
/// headers: 10 seconds. Each read waits only for the time left.
pub(super) const READ_TIMEOUT: Duration = Duration::from_secs(10);

/// The most time one write waits for the client to read: 10 seconds.
pub(super) const WRITE_TIMEOUT: Duration = Duration::from_secs(10);

/// The most connections served at once, server-sent streams included.
pub(super) const CONNECTIONS: usize = 64;

/// The limits of the loopback surface. Production takes
/// [`Limits::DEFAULT`]; the tests inject small ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Limits {
    /// Bytes of the request line: [`REQUEST_LINE_BYTES`].
    pub(super) request_line_bytes: usize,
    /// Bytes of the header section: [`HEADER_BYTES`].
    pub(super) header_bytes: usize,
    /// Time to read the request: [`READ_TIMEOUT`].
    pub(super) read_timeout: Duration,
    /// Time one write waits: [`WRITE_TIMEOUT`].
    pub(super) write_timeout: Duration,
    /// Connections served at once: [`CONNECTIONS`].
    pub(super) connections: usize,
}

impl Limits {
    pub(super) const DEFAULT: Limits = Limits {
        request_line_bytes: REQUEST_LINE_BYTES,
        header_bytes: HEADER_BYTES,
        read_timeout: READ_TIMEOUT,
        write_timeout: WRITE_TIMEOUT,
        connections: CONNECTIONS,
    };

    /// One line of the request within `room` bytes, as text. Past `room`
    /// it is `over`; a read that times out is the read timeout's refusal;
    /// any other failure, or bytes that are not UTF-8, leave the request
    /// unreadable, as `read_line` left it.
    pub(super) fn line(
        &self,
        reader: &mut impl BufRead,
        room: usize,
        over: Exceeded,
    ) -> Result<String, Unread> {
        match line_within(reader, room) {
            Ok(Some(line)) => String::from_utf8(line).map_err(|_| Unread::Unreadable),
            Ok(None) => Err(Unread::Exceeded(over)),
            Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                Err(Unread::Exceeded(Exceeded::Timeout {
                    millis: self.read_timeout.as_millis(),
                }))
            }
            Err(_) => Err(Unread::Unreadable),
        }
    }
}

/// A limit a client exceeded, in the words its response carries: the limit
/// and its unit, never the client's bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Exceeded {
    RequestLine { limit: usize },
    Headers { limit: usize },
    Timeout { millis: u128 },
    Connections { limit: usize },
}

impl fmt::Display for Exceeded {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Exceeded::RequestLine { limit } => write!(
                formatter,
                "the request line exceeds the limit of {limit} bytes"
            ),
            Exceeded::Headers { limit } => write!(
                formatter,
                "the request headers exceed the limit of {limit} bytes"
            ),
            Exceeded::Timeout { millis } => write!(
                formatter,
                "the request did not arrive within the limit of {millis} milliseconds"
            ),
            Exceeded::Connections { limit } => write!(
                formatter,
                "the surface is serving its limit of {limit} connections"
            ),
        }
    }
}

impl Exceeded {
    /// The response that refuses it, with the status HTTP gives it.
    pub(super) fn response(&self) -> Response {
        Response {
            status: match self {
                Exceeded::RequestLine { .. } => "414 URI Too Long",
                Exceeded::Headers { .. } => "431 Request Header Fields Too Large",
                Exceeded::Timeout { .. } => "408 Request Timeout",
                Exceeded::Connections { .. } => "503 Service Unavailable",
            },
            content_type: "application/json",
            body: json!({"error": self.to_string()}).to_string(),
        }
    }
}

/// Why a request was not read.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Unread {
    /// Past a limit: answered with its refusal.
    Exceeded(Exceeded),
    /// A read failed, or its bytes were not UTF-8: closed unanswered.
    Unreadable,
}

/// One line of `reader`, its newline included, of which no more than one
/// byte past `room` is ever read: `None` when that byte arrives. At EOF the
/// line is what arrived, empty when nothing did, and an interrupted read
/// is retried, as `read_line` reads one.
fn line_within(reader: &mut impl BufRead, room: usize) -> io::Result<Option<Vec<u8>>> {
    let mut line = Vec::new();
    let most = u64::try_from(room).unwrap_or(u64::MAX).saturating_add(1);
    Read::take(&mut *reader, most).read_until(b'\n', &mut line)?;
    Ok((line.len() <= room).then_some(line))
}

/// A client's request bytes, read before its deadline: each read waits only
/// for what is left of `timeout` since the connection was accepted, and a
/// read with nothing left times out without waiting.
pub(super) struct Deadline<'s> {
    stream: &'s TcpStream,
    start: Instant,
    timeout: Duration,
}

impl<'s> Deadline<'s> {
    pub(super) fn new(stream: &'s TcpStream, timeout: Duration) -> Self {
        Deadline {
            stream,
            start: Instant::now(),
            timeout,
        }
    }
}

impl Read for Deadline<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let left = self.timeout.saturating_sub(self.start.elapsed());
        if left.is_zero() {
            return Err(ErrorKind::TimedOut.into());
        }
        self.stream.set_read_timeout(Some(left))?;
        let mut stream = self.stream;
        stream.read(buffer)
    }
}

/// The connections being served, against their limit. Only the acceptor
/// takes a slot; the thread it starts gives it back when its [`Slot`]
/// drops, on every return and on unwind.
pub(super) struct Slots {
    held: Arc<AtomicUsize>,
    limit: usize,
}

impl Slots {
    pub(super) fn new(limit: usize) -> Self {
        Slots {
            held: Arc::new(AtomicUsize::new(0)),
            limit,
        }
    }

    /// A slot, or none when `limit` are held.
    pub(super) fn take(&self) -> Option<Slot> {
        self.held
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |held| {
                (held < self.limit).then_some(held + 1)
            })
            .ok()
            .map(|_| Slot(Arc::clone(&self.held)))
    }
}

/// One connection's hold on [`Slots`], given back when it drops.
pub(super) struct Slot(Arc<AtomicUsize>);

impl Drop for Slot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}
