//! The loopback surface's limits (#433): a request's bytes and time, a
//! write's wait and the connections served at once, each held with a small
//! injected limit over a real loopback socket. The HTTP fixtures the other
//! route tests share live here too.

use super::*;
use std::io::{Cursor, Read};
use std::net::Shutdown;
use std::time::{Duration, Instant};

pub(super) struct BrokenReader;

impl Read for BrokenReader {
    fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("broken request"))
    }
}

impl BufRead for BrokenReader {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        Err(std::io::Error::other("broken request"))
    }

    fn consume(&mut self, _: usize) {}
}

pub(super) struct BrokenWriter;

impl Write for BrokenWriter {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("broken response"))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[derive(Default)]
pub(super) struct FailAfterOneWrite {
    writes: usize,
}

impl Write for FailAfterOneWrite {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.writes += 1;
        if self.writes == 1 {
            Ok(bytes.len())
        } else {
            Err(std::io::Error::other("client disconnected"))
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// One connection served under the default limits: `request` written and
/// the write side shut, the reply read to the server's close.
pub(super) fn exchange(db: PathBuf, request: &str, sse_limit: Option<usize>) -> String {
    let request = Request {
        bytes: request.as_bytes(),
        close: true,
    };
    connection(db, Limits::DEFAULT, request, sse_limit)
}

/// Small limits, each test overriding the one it holds.
fn small() -> Limits {
    Limits {
        request_line_bytes: 32,
        header_bytes: 64,
        read_timeout: Duration::from_millis(300),
        write_timeout: Duration::from_secs(5),
        connections: 2,
    }
}

/// No journal: `/` and the server-sent head need none, and nothing reads
/// one into being.
fn no_journal() -> PathBuf {
    PathBuf::from("missing.db")
}

const PAGE_REQUEST: &[u8] = b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";

/// The whole reply a refusal writes: its status and its fixed body.
fn refusal(status: &str, error: &str) -> String {
    let body = json!({ "error": error }).to_string();
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nCache-Control: no-store\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

/// A client that gives up on a reply after `wait`, so a server that never
/// answers fails the test instead of hanging it.
fn client(address: std::net::SocketAddr, wait: Duration) -> TcpStream {
    let stream = TcpStream::connect(address).unwrap();
    stream.set_read_timeout(Some(wait)).unwrap();
    stream
}

/// What a client sends: its bytes, and whether it then shuts its write side.
struct Request<'r> {
    bytes: &'r [u8],
    close: bool,
}

/// One connection served under `limits`, the reply read to the server's
/// close.
fn connection(db: PathBuf, limits: Limits, request: Request, sse_limit: Option<usize>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        serve_client_with_limit(db, stream, limits, sse_limit);
    });
    let mut client = client(address, Duration::from_secs(10));
    client.write_all(request.bytes).unwrap();
    if request.close {
        client.shutdown(Shutdown::Write).unwrap();
    }
    let mut reply = String::new();
    client.read_to_string(&mut reply).unwrap();
    server.join().unwrap();
    reply
}

/// One connection with no journal under `limits`.
fn served(limits: Limits, bytes: &[u8], close: bool) -> String {
    connection(no_journal(), limits, Request { bytes, close }, None)
}

#[test]
fn the_limits_and_their_units_are_declared() {
    assert_eq!(
        Limits::DEFAULT,
        Limits {
            request_line_bytes: 8192,
            header_bytes: 32768,
            read_timeout: Duration::from_secs(10),
            write_timeout: Duration::from_secs(10),
            connections: 64,
        }
    );
}

#[test]
fn an_oversized_request_line_is_refused_before_it_is_held() {
    let request = format!(
        "GET /{} HTTP/1.1\r\nHost: localhost\r\n\r\n",
        "a".repeat(40)
    );
    assert_eq!(
        served(small(), request.as_bytes(), false),
        refusal(
            "414 URI Too Long",
            "the request line exceeds the limit of 32 bytes"
        )
    );
}

#[test]
fn an_oversized_header_section_with_no_blank_line_is_refused() {
    let request = format!(
        "GET / HTTP/1.1\r\nHost: localhost\r\nX-Pad: {}",
        "a".repeat(80)
    );
    assert_eq!(
        served(small(), request.as_bytes(), false),
        refusal(
            "431 Request Header Fields Too Large",
            "the request headers exceed the limit of 64 bytes"
        )
    );
}

#[test]
fn a_client_that_never_sends_or_stalls_is_refused_at_the_read_timeout() {
    let timeout = refusal(
        "408 Request Timeout",
        "the request did not arrive within the limit of 300 milliseconds",
    );
    assert_eq!(served(small(), b"", false), timeout);
    assert_eq!(
        served(small(), b"GET / HTTP/1.1\r\nHost: loc", false),
        timeout
    );
    // A zero timeout reads nothing, not even an end of input; the client
    // sends nothing, so no unread request resets its reply.
    let zero = Limits {
        read_timeout: Duration::ZERO,
        ..small()
    };
    assert_eq!(
        served(zero, b"", true),
        refusal(
            "408 Request Timeout",
            "the request did not arrive within the limit of 0 milliseconds"
        )
    );
}

/// A zero write timeout admits no write, so the connection is closed
/// unanswered rather than answered without one.
#[test]
fn a_zero_write_timeout_closes_the_connection_unanswered() {
    let zero = Limits {
        write_timeout: Duration::ZERO,
        ..small()
    };
    assert_eq!(served(zero, b"", true), "");
}

/// `connections` served at once, each given five seconds for its request.
fn capped(connections: usize) -> Limits {
    Limits {
        connections,
        read_timeout: Duration::from_secs(5),
        ..small()
    }
}

/// A silent client past a cap of `limit` is answered 503 well inside the
/// five seconds a served one would wait for its request.
fn refused_at_once(address: std::net::SocketAddr, limit: usize) {
    let mut extra = client(address, Duration::from_secs(2));
    let mut reply = String::new();
    extra.read_to_string(&mut reply).unwrap();
    assert_eq!(
        reply,
        refusal(
            "503 Service Unavailable",
            &format!("the surface is serving its limit of {limit} connections")
        )
    );
}

#[test]
fn connections_past_the_cap_are_refused_at_once_while_the_others_are_served() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        serve_listener(no_journal(), listener, false, Some(3), capped(2), |_| {})
    });
    // Accepted in connect order: the first two hold both slots, silent.
    let held: Vec<TcpStream> = (0..2)
        .map(|_| client(address, Duration::from_secs(5)))
        .collect();
    // The third is answered while they are open, so it was never given a
    // thread that waited for its request.
    refused_at_once(address, 2);
    for mut held in held {
        held.write_all(PAGE_REQUEST).unwrap();
        held.shutdown(Shutdown::Write).unwrap();
        let mut page = String::new();
        held.read_to_string(&mut page).unwrap();
        assert!(page.starts_with("HTTP/1.1 200 OK\r\n"), "{page}");
    }
    server.join().unwrap().unwrap();
}

#[test]
fn a_disconnected_sse_client_frees_its_slot() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        serve_listener(no_journal(), listener, false, None, capped(1), |_| {})
    });

    let mut watcher = client(address, Duration::from_secs(5));
    watcher
        .write_all(b"GET /sse/r1 HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .unwrap();
    let mut events = BufReader::new(watcher.try_clone().unwrap());
    let mut line = String::new();
    while line != "data: {\"seq\":0}\n" {
        line.clear();
        assert_ne!(events.read_line(&mut line).unwrap(), 0, "the stream ended");
    }
    // The open stream holds the one slot.
    refused_at_once(address, 1);

    // Gone: the stream's next heartbeats fail, its thread returns and the
    // slot comes back, within two polls.
    drop(events);
    drop(watcher);
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let mut next = client(address, Duration::from_secs(5));
        // A refused client may be reset for the request it sent.
        let _ = next.write_all(PAGE_REQUEST);
        let _ = next.shutdown(Shutdown::Write);
        let mut page = String::new();
        let _ = next.read_to_string(&mut page);
        if page.starts_with("HTTP/1.1 200 OK\r\n") {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "the slot never came back: {page}"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[test]
fn slots_admit_up_to_their_limit_and_zero_admits_none() {
    assert!(Slots::new(0).take().is_none());
    let slots = Slots::new(1);
    let first = slots.take();
    assert!(first.is_some());
    assert!(slots.take().is_none());
    drop(first);
    assert!(slots.take().is_some());
}

/// A line is held to its room exactly: one that fits is read, one byte
/// more is refused, and a zero limit admits nothing. Reading in chunks
/// smaller than the line changes none of it.
#[test]
fn a_line_is_held_to_its_room_exactly() {
    let read = |request: &[u8], limits: Limits| {
        read_request(&mut BufReader::with_capacity(4, request), &limits)
    };
    let request = b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
    let exact = Limits {
        request_line_bytes: 16,
        header_bytes: 19,
        ..small()
    };
    let admitted = ("GET".to_string(), "/".to_string(), Some("localhost".into()));
    assert_eq!(read(request, exact), Ok(admitted));
    assert_eq!(
        read(
            request,
            Limits {
                request_line_bytes: 15,
                ..exact
            }
        ),
        Err(Unread::Exceeded(Exceeded::RequestLine { limit: 15 }))
    );
    assert_eq!(
        read(
            request,
            Limits {
                header_bytes: 18,
                ..exact
            }
        ),
        Err(Unread::Exceeded(Exceeded::Headers { limit: 18 }))
    );

    let zero = Limits {
        request_line_bytes: 0,
        header_bytes: 0,
        ..small()
    };
    assert_eq!(
        read(request, zero),
        Err(Unread::Exceeded(Exceeded::RequestLine { limit: 0 }))
    );
    assert_eq!(
        read(
            b"GET / HTTP/1.1\r\n\r\n",
            Limits {
                header_bytes: 0,
                ..exact
            }
        ),
        Err(Unread::Exceeded(Exceeded::Headers { limit: 0 }))
    );
    // Nothing sent is nothing admitted: an empty request line, which the
    // method check forbids.
    assert_eq!(read(b"", zero), Ok((String::new(), "/".to_string(), None)));
}

#[test]
fn bytes_that_are_not_utf8_leave_the_request_unreadable() {
    for request in [
        &b"GET /\xff HTTP/1.1\r\n\r\n"[..],
        &b"GET / HTTP/1.1\r\nHost: \xff\r\n\r\n"[..],
    ] {
        assert_eq!(
            read_request(&mut Cursor::new(request), &Limits::DEFAULT),
            Err(Unread::Unreadable)
        );
    }
}
