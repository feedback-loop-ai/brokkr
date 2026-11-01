//! The filtered protocol (decision 0065 slice two, U6e; MB3, CC2, CR3,
//! SC4) over real anonymous pipes: a fake server answers on a thread of
//! its own, a harness speaks on another, and the ledger is held in memory
//! with an append's fault injected, then settled as `serve` settles it.
//! The live box is the CLI suite's.

use std::cell::RefCell;
use std::io::{BufRead, BufReader, ErrorKind, PipeReader, PipeWriter, Read, Write};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use brokkr_protocol::broker::ledger::{
    recover, Denial, Disposition, Failure, Invalid, Ledger, Outcome, Unrecorded,
};
use brokkr_protocol::broker::Plan;
use brokkr_protocol::secret::BoundSecret;
use serde_json::{json, Value};

use super::calls::{served, served_within};
use super::output::Edge;
use super::{started, Catalog, Fatal, Line, Lines, Refused};
use super::{NOTICES_MAX, REQUEST_MAX, RESPONSE_MAX};
use crate::broker::session::lifetime::settled;
use crate::broker::session::stderr::Drain;
use crate::broker::session::{Ended, Startup};

mod delivery;
mod hangup;
mod masking;
mod retention;

/// The plan every test serves, `@` its digests: the capability
/// `library-docs`, its dialect pinned at 1.4.2, granting `lookup` and
/// `read`.
const PLAN: &str = concat!(
    r#"{"owner": {"repo": "@", "run": "run-1", "effect": "e", "attempt": "a", "#,
    r#""site": "s", "instance": "s#0"}, "server": "cap-library-docs", "#,
    r#""capability": "library-docs", "dialect": {"name": "docs-mcp", "digest": "@", "#,
    r#""definition": "@", "version": "1.4.2"}, "connection": {"argv": ["/opt/docs"]}, "#,
    r#""tools": ["lookup", "read"], "restrictions": {}, "retained": false, "secrets": [], "#,
    r#""clearance": {"dialect": "@", "policy": "@"}, "box": {"reach": {"writable": [], "#,
    r#""readable": []}, "executable": "/opt/docs", "tree": {"kind": "system"}, "#,
    r#""sources": {"entries": 1, "mounts": 1, "digest": "@"}, "writers": {"uids": [1], "#,
    r#""privilege": "confined"}, "network": "isolated", "environment": [], "#,
    r#""bootstrap": {"path": "/", "digest": "@"}, "excluded": {"store": "/s", "control": []}}}"#,
);

/// The sealed plan's digest.
const DIGEST: &str = "5ea1ed0000000000000000000000000000000000000000000000000000000000";

fn plan() -> Plan {
    serde_json::from_str(&PLAN.replace('@', &"b".repeat(64))).unwrap()
}

/// Never cancelled.
fn live() -> bool {
    false
}

/// The attempt `within` from now, never cancelled.
fn within(within: Duration) -> Startup<'static> {
    (Instant::now() + within, &live)
}

/// An anonymous pipe, as every test's server and harness speak over: the
/// one place a run sizes them all, as U6e's evidence did to macOS's 16 KiB.
fn pipe() -> (PipeReader, PipeWriter) {
    std::io::pipe().unwrap()
}

/// How long a test's side waits on a pipe that has nothing for it, or
/// takes nothing from it, before it tries again.
const TURN: Duration = Duration::from_micros(200);

/// Whether all of `bytes` was written to the non-blocking `to` by `until`;
/// not where the reader ended first, or the deadline came: no write waits
/// on a broker that has stopped reading.
fn offered(to: &mut PipeWriter, mut bytes: &[u8], until: Instant) -> bool {
    while !bytes.is_empty() & (Instant::now() < until) {
        match to.write(bytes) {
            Ok(taken) => bytes = &bytes[taken..],
            Err(error) if error.kind() == ErrorKind::WouldBlock => std::thread::sleep(TURN),
            Err(_) => break,
        }
    }
    bytes.is_empty()
}

/// What a fake server does once it has read a frame: reads on, holds its
/// input unread, closes it, or is gone: closes its input before it says
/// `answer`, so that no scheduling lets a forwarding in by this reader,
/// and ends its output half a second after, since a child another test
/// has forked and not yet exec'd holds every pipe's read end until it
/// does: a forwarding that pipe took is then ended, well within its bound.
#[derive(Clone)]
enum Then {
    Read,
    Hold,
    Close,
    Gone(Value),
}

/// A fake server's way with each frame it reads, given its output, which
/// it may close.
type Script = Box<dyn FnMut(&Value, &mut Option<PipeWriter>) -> Then + Send>;

/// What a fake server leaves once joined: each frame it read, and its
/// input and output, held until then unless it closed them.
type Faked = (
    Vec<Value>,
    Option<BufReader<PipeReader>>,
    Option<PipeWriter>,
);

/// A server running `script` on a thread of its own: the broker's ends of
/// its standard input and output, and the thread.
fn fake(mut script: Script) -> (PipeWriter, PipeReader, JoinHandle<Faked>) {
    let ((stdin, input), (output, stdout)) = (pipe(), pipe());
    let thread = std::thread::spawn(move || {
        let (mut stdin, mut stdout) = (Some(BufReader::new(stdin)), Some(stdout));
        let mut frames = Vec::new();
        while let Some(reader) = stdin.as_mut() {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap_or(0) == 0 {
                break;
            }
            let frame: Value = serde_json::from_str(&line).unwrap();
            let then = script(&frame, &mut stdout);
            frames.push(frame);
            match then {
                Then::Read => {}
                Then::Hold => break,
                Then::Close => stdin = None,
                Then::Gone(answer) => {
                    stdin = None;
                    say(&mut stdout, &answer);
                    std::thread::sleep(Duration::from_millis(500));
                    stdout = None;
                }
            }
        }
        (frames, stdin, stdout)
    });
    (input, output, thread)
}

/// Write `frame` to a fake server's `output`, where it is open.
fn say(output: &mut Option<PipeWriter>, frame: &Value) -> bool {
    let line = format!("{frame}\n");
    let written = output.as_mut().map(|out| out.write_all(line.as_bytes()));
    matches!(written, Some(Ok(())))
}

/// The tools a well-behaved server lists: both granted ones beside one
/// that is not, each described by data the broker passes on whole, a quote
/// escaped in one.
fn tools() -> Vec<Value> {
    vec![
        json!({"name": "lookup", "description": "finds a \"page\"", "inputSchema": {"type": "object"}}),
        json!({"name": "admin", "inputSchema": {"type": "object"}}),
        json!({"name": "read", "inputSchema": {"type": "object", "required": ["page"]}}),
    ]
}

/// The result answering `id`.
fn result(id: &Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

/// The error answering `id`.
fn refusal(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

/// What a server pinned at 1.4.2 says to initialize, at MB3's version.
fn hello() -> Value {
    let info = json!({"name": "docs", "version": "1.4.2"});
    json!({"protocolVersion": "2025-06-18", "capabilities": {"tools": {}}, "serverInfo": info})
}

/// How a well-behaved server answers `frame`: initialize with [`hello`],
/// its listing with one page of [`tools`], and a call with its tool's
/// name; a notification with nothing.
fn answer(frame: &Value) -> Option<Value> {
    let answered = match frame["method"].as_str()? {
        "initialize" => hello(),
        "tools/list" => json!({"tools": tools()}),
        "tools/call" => json!({"content": [{"type": "text", "text": frame["params"]["name"]}]}),
        _ => return None,
    };
    Some(result(&frame["id"], answered))
}

/// A well-behaved server, but where `instead` answers a frame otherwise.
fn mcp_but(
    mut instead: impl FnMut(&Value, &mut Option<PipeWriter>) -> Option<Then> + Send + 'static,
) -> Script {
    Box::new(move |frame, output| {
        instead(frame, output).unwrap_or_else(|| {
            answer(frame).map(|answer| say(output, &answer));
            Then::Read
        })
    })
}

/// A well-behaved server.
fn mcp() -> Script {
    mcp_but(|_, _| None)
}

/// What the server `script` runs, started within `within`, offers: the
/// catalog's tools, or the session's fatal ending; and each frame it read.
fn startup(script: Script, within: Duration) -> (Result<Vec<Value>, Fatal>, Vec<Value>) {
    let plan = plan();
    let (mut input, mut output, server) = fake(script);
    let mut stderr = Drain::new(pipe().0, &[]);
    let started = started(
        (&mut input, &mut output),
        &plan,
        self::within(within),
        &mut stderr,
    );
    let offered = started.map(|(_, Catalog(tools))| tools.iter().map(|tool| json!(tool)).collect());
    drop((input, output));
    (offered, server.join().unwrap().0)
}

/// The frames the broker sends a server it initializes, and asks for its
/// first page.
fn initialization() -> Vec<Value> {
    let client = json!({"name": "brokkr-broker", "version": env!("CARGO_PKG_VERSION")});
    let params = json!({"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": client});
    vec![
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": params}),
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}}),
    ]
}

/// The tools the plan's grant offers, as a well-behaved server describes
/// them.
fn granted() -> Vec<Value> {
    let tools = tools();
    vec![tools[0].clone(), tools[2].clone()]
}

#[test]
fn each_cause_reads_in_mb3s_words_and_latches_through_the_ledgers_own() {
    let ungranted = Refused::Ungranted {
        tool: "admin".into(),
        capability: "library-docs".into(),
    };
    let refused = [
        (
            ungranted,
            -32602,
            "tool 'admin' is not granted to capability 'library-docs'",
        ),
        (
            Refused::Method,
            -32601,
            "MCP method is not admitted by this broker",
        ),
        (
            Refused::Limit,
            -32600,
            "MCP request exceeds the broker limit",
        ),
        (Refused::Invalid, -32600, "MCP tool call request is invalid"),
        (
            Refused::Active,
            -32600,
            "MCP broker already has an active call",
        ),
    ];
    for (refused, code, text) in refused {
        assert_eq!(
            (refused.code(), refused.to_string()),
            (code, text.to_string())
        );
    }
    let fatal = [
        (
            Fatal::Failed(Failure::Timeout),
            Failure::Timeout,
            "MCP server call timed out",
        ),
        (
            Fatal::Unsupported,
            Failure::Protocol,
            "MCP protocol version is not supported by this broker",
        ),
        (
            Fatal::Missing("read".into()),
            Failure::Protocol,
            "MCP server does not offer granted tool 'read'",
        ),
        (
            Fatal::Catalog,
            Failure::Response,
            "MCP tool catalog exceeds the broker limit",
        ),
    ];
    for (fatal, failure, text) in fatal {
        assert_eq!(Ended::Failed(fatal.clone()).to_string(), text);
        assert_eq!(
            (fatal.failure(), fatal.to_string()),
            (failure, text.to_string())
        );
    }
}

#[test]
fn startup_initializes_lists_and_offers_exactly_the_grant() {
    let (offered, frames) = startup(mcp(), Duration::from_secs(5));
    assert_eq!(offered, Ok(granted()));
    assert_eq!(frames, initialization());
}

#[test]
fn a_server_off_its_pinned_version_or_mb3s_protocol_ends_the_session_before_any_listing() {
    let cases = [
        (
            "serverInfo",
            json!({"name": "docs", "version": "1.4.3"}),
            Fatal::Failed(Failure::Version),
        ),
        ("protocolVersion", json!("2024-11-05"), Fatal::Unsupported),
    ];
    for (member, value, fatal) in cases {
        let script = mcp_but(move |frame, output| {
            let mut said = hello();
            said[member] = value.clone();
            (frame["method"] == "initialize").then(|| {
                say(output, &result(&frame["id"], said));
                Then::Read
            })
        });
        let (offered, frames) = startup(script, Duration::from_secs(5));
        assert_eq!((member, offered), (member, Err(fatal)));
        assert_eq!((member, frames), (member, initialization()[..1].to_vec()));
    }
}

/// A server whose catalog is `pages`, each a page of tools by name and
/// the cursor it names next.
fn paged(pages: Vec<(Vec<&'static str>, Option<&'static str>)>) -> Script {
    let mut pages = pages.into_iter();
    mcp_but(move |frame, output| {
        (frame["method"] == "tools/list").then(|| {
            let (names, next) = pages.next().unwrap();
            let tools: Vec<Value> = names.iter().map(|name| json!({"name": name})).collect();
            let page = json!({"tools": tools, "nextCursor": next});
            say(output, &result(&frame["id"], page));
            Then::Read
        })
    })
}

#[test]
fn the_whole_catalog_is_paged_and_filtered_and_a_granted_tool_lacking_or_a_name_twice_ends_it() {
    let pages = vec![(vec!["lookup", "admin"], Some("c1")), (vec!["read"], None)];
    let (offered, frames) = startup(paged(pages), Duration::from_secs(5));
    assert_eq!(
        offered,
        Ok(vec![json!({"name": "lookup"}), json!({"name": "read"})])
    );
    let next =
        json!({"jsonrpc": "2.0", "id": 3, "method": "tools/list", "params": {"cursor": "c1"}});
    assert_eq!(frames[3..], [next]);
    let lacking = vec![(vec!["lookup", "admin"], None)];
    let (offered, _) = startup(paged(lacking), Duration::from_secs(5));
    assert_eq!(offered, Err(Fatal::Missing("read".into())));
    // A name listed twice, granted or not, on one page or across two.
    let twice = [
        vec![
            (vec!["lookup", "admin"], Some("c1")),
            (vec!["read", "lookup"], None),
        ],
        vec![(vec!["lookup", "admin", "read", "admin"], None)],
    ];
    for (case, pages) in twice.into_iter().enumerate() {
        let (offered, _) = startup(paged(pages), Duration::from_secs(5));
        assert_eq!((case, offered), (case, Err(Fatal::Catalog)));
    }
}

/// A server whose catalog is `pages` pages, each naming the next, and
/// `tools` tools in all, each named once, `read` and `lookup` among them;
/// or whose second page names its first page's cursor again.
fn catalog(pages: usize, tools: usize, repeated: bool) -> Script {
    let others = (2..tools).map(|at| &*Box::leak(format!("other-{at}").into_boxed_str()));
    let names: Vec<&'static str> = ["lookup", "read"].into_iter().chain(others).collect();
    let mut chunks: Vec<Vec<&'static str>> = vec![Vec::new(); pages];
    for (at, name) in names.into_iter().enumerate() {
        chunks[at % pages].push(name);
    }
    let cursors = [
        "c1", "c2", "c3", "c4", "c5", "c6", "c7", "c8", "c9", "ca", "cb",
    ];
    let cursor = |at: usize| match (at + 1 < pages, repeated) {
        (false, _) => None,
        (true, true) => Some("c1"),
        (true, false) => Some(&*Box::leak(
            format!("{}-{at}", cursors[at % 11]).into_boxed_str(),
        )),
    };
    let pages = chunks
        .into_iter()
        .enumerate()
        .map(|(at, names)| (names, cursor(at)))
        .collect();
    paged(pages)
}

#[test]
fn the_catalogs_page_tool_and_cursor_bounds_hold_exactly() {
    let catalog_limit = Err(Fatal::Catalog);
    let (bounded, _) = startup(catalog(32, 1024, false), Duration::from_secs(10));
    assert_eq!(bounded.map(|tools| tools.len()), Ok(2));
    let (pages, _) = startup(catalog(33, 33, false), Duration::from_secs(10));
    assert_eq!(pages.map(|tools| tools.len()), catalog_limit);
    let (tools, _) = startup(catalog(2, 1025, false), Duration::from_secs(10));
    assert_eq!(tools.map(|tools| tools.len()), catalog_limit);
    let (cursor, frames) = startup(catalog(3, 3, true), Duration::from_secs(10));
    assert_eq!(cursor.map(|tools| tools.len()), catalog_limit);
    assert_eq!(frames.len(), 4);
}

/// A server that answers initialize with the line `said`, then stops.
fn initializing(said: String) -> Script {
    mcp_but(move |frame, output| {
        (frame["method"] == "initialize").then(|| {
            output.as_mut().unwrap().write_all(said.as_bytes()).unwrap();
            Then::Read
        })
    })
}

#[test]
fn a_response_must_answer_the_one_request_outstanding_by_its_exact_typed_id() {
    let hello = hello();
    let cases = [
        (
            "a string for a number",
            json!({"jsonrpc": "2.0", "id": "1", "result": hello}),
        ),
        (
            "another id",
            json!({"jsonrpc": "2.0", "id": 2, "result": hello}),
        ),
        ("an error", refusal(json!(1), -32603, "no")),
        (
            "both bodies",
            json!({"jsonrpc": "2.0", "id": 1, "result": hello, "error": {}}),
        ),
        (
            "an extra member",
            json!({"jsonrpc": "2.0", "id": 1, "result": hello, "x": 1}),
        ),
        (
            "another version",
            json!({"jsonrpc": "1.0", "id": 1, "result": hello}),
        ),
        (
            "no server info",
            result(&json!(1), json!({"protocolVersion": "2025-06-18"})),
        ),
        ("no id", json!({"jsonrpc": "2.0", "result": hello})),
        (
            "a null method",
            json!({"jsonrpc": "2.0", "id": 1, "method": null, "result": hello}),
        ),
        (
            "server info by position",
            result(
                &json!(1),
                json!({"protocolVersion": "2025-06-18", "serverInfo": ["1.4.2"]}),
            ),
        ),
    ];
    for (case, said) in cases {
        let (offered, _) = startup(initializing(format!("{said}\n")), Duration::from_secs(5));
        assert_eq!(
            (case, offered),
            (case, Err(Fatal::Failed(Failure::Protocol)))
        );
    }
    let listing = |tools: Value| {
        mcp_but(move |frame, output| {
            (frame["method"] == "tools/list").then(|| {
                say(output, &result(&frame["id"], json!({"tools": tools})));
                Then::Read
            })
        })
    };
    let nameless = listing(json!([{"title": "x"}]));
    let positional = listing(json!([["lookup"], ["read"]]));
    // A ping by position, before the answer it would otherwise not hold up.
    let pinged = json!(["2.0", 7, "ping", {}]);
    let pinged = initializing(format!("{pinged}\n{}\n", result(&json!(1), hello.clone())));
    let ended = mcp_but(|frame, output| {
        (frame["method"] == "initialize").then(|| {
            *output = None;
            Then::Read
        })
    });
    for (case, script, cause) in [
        (
            "not JSON",
            initializing("{\"jsonrpc\"\n".into()),
            Failure::Protocol,
        ),
        ("a request by position", pinged, Failure::Protocol),
        ("a nameless tool", nameless, Failure::Protocol),
        ("tools by position", positional, Failure::Protocol),
        ("its output's end", ended, Failure::Protocol),
    ] {
        let (offered, _) = startup(script, Duration::from_secs(5));
        assert_eq!((case, offered), (case, Err(Fatal::Failed(cause))));
    }
}

#[test]
fn a_servers_requests_never_reach_the_harness_and_every_frame_spends_its_budget() {
    // A ping, a request for sampling and a notification before the answer:
    // the ping answered with nothing, the request refused, the notification
    // dropped, and initialization goes on.
    let asking = mcp_but(|frame, output| {
        (frame["method"] == "initialize").then(|| {
            say(
                output,
                &json!({"jsonrpc": "2.0", "id": "p", "method": "ping"}),
            );
            let sampling = json!({"jsonrpc": "2.0", "id": 7, "method": "sampling/createMessage"});
            say(output, &sampling);
            say(
                output,
                &json!({"jsonrpc": "2.0", "method": "notifications/tools/list_changed"}),
            );
            say(output, &result(&frame["id"], hello()));
            Then::Read
        })
    });
    let (offered, frames) = startup(asking, Duration::from_secs(5));
    assert_eq!(offered, Ok(granted()));
    let method = "MCP method is not admitted by this broker";
    let mut expected = initialization();
    expected.splice(
        1..1,
        [
            result(&json!("p"), json!({})),
            refusal(json!(7), -32601, method),
        ],
    );
    assert_eq!(frames, expected);
    // Notifications of exactly the budget pass; one byte more is the
    // response limit.
    for (extra, ended) in [(0, Ok(2)), (1, Err(Fatal::Failed(Failure::Response)))] {
        let noisy = mcp_but(move |frame, output| {
            (frame["method"] == "initialize").then(|| {
                noise(output, NOTICES_MAX + extra);
                say(output, &result(&frame["id"], hello()));
                Then::Read
            })
        });
        let (offered, _) = startup(noisy, Duration::from_secs(10));
        assert_eq!((extra, offered.map(|tools| tools.len())), (extra, ended));
    }
}

/// Write to a server's `output` notifications whose frames hold exactly
/// `bytes` in all, at least one frame's worth.
fn noise(output: &mut Option<PipeWriter>, bytes: usize) {
    let notice = |pad: usize| {
        let params = json!({"pad": "x".repeat(pad)});
        json!({"jsonrpc": "2.0", "method": "notifications/message", "params": params})
    };
    let size = notice(0).to_string().len();
    let pad = 64 << 10;
    for _ in 0..bytes / (size + pad) {
        say(output, &notice(pad));
    }
    say(output, &notice(bytes % (size + pad) - size));
}

/// A server whose catalog is two pages, the first naming the second, each
/// listing `tools` and answered after so many bytes of notifications.
fn pages(first: (usize, Vec<Value>), second: (usize, Vec<Value>)) -> Script {
    let mut pages = [(first, Some("c1")), (second, None)].into_iter();
    mcp_but(move |frame, output| {
        (frame["method"] == "tools/list").then(|| {
            let ((bytes, tools), next) = pages.next().unwrap();
            (bytes > 0).then(|| noise(output, bytes));
            say(
                output,
                &result(&frame["id"], json!({"tools": tools, "nextCursor": next})),
            );
            Then::Read
        })
    })
}

#[test]
fn a_listings_pages_share_one_budget_of_frames_that_answer_nothing() {
    // Half the budget before the first page, the rest before the second:
    // exactly the budget passes, one byte more is the response limit.
    let half = NOTICES_MAX / 2;
    for (extra, ended) in [(0, Ok(2)), (1, Err(Fatal::Failed(Failure::Response)))] {
        let noisy = pages((half, granted()), (NOTICES_MAX - half + extra, vec![]));
        let (offered, frames) = startup(noisy, Duration::from_secs(10));
        assert_eq!((extra, offered.map(|tools| tools.len())), (extra, ended));
        assert_eq!((extra, frames.len()), (extra, 4));
    }
}

/// The tool `name`, described by `bytes` of text.
fn described(name: &str, bytes: usize) -> Value {
    json!({"name": name, "description": "d".repeat(bytes)})
}

#[test]
fn a_listing_of_the_grant_past_mb3s_bound_on_a_response_ends_the_session() {
    // Each granted tool on a page of its own, each page within the bound,
    // and the listing together exactly the bound; one byte more is the
    // response limit, never a listing delivered.
    let empty = json!({"tools": [described("lookup", 0), described("read", 0)]});
    let (lookup, empty) = (5 << 20, empty.to_string().len());
    let read = RESPONSE_MAX - empty - lookup;
    for (extra, ended) in [(0, Ok(2)), (1, Err(Fatal::Failed(Failure::Response)))] {
        let first = (0, vec![described("lookup", lookup)]);
        let script = pages(first, (0, vec![described("read", read + extra)]));
        let (offered, _) = startup(script, Duration::from_secs(20));
        assert_eq!((extra, offered.map(|tools| tools.len())), (extra, ended));
    }
}

/// A server whose listing is padded to `bytes` and nests `depth` deep, its
/// arrays within the frame's object and its result.
fn sized(bytes: usize, depth: usize) -> Script {
    mcp_but(move |frame, output| {
        (frame["method"] == "tools/list").then(|| {
            let nested = "[".repeat(depth - 2) + &"]".repeat(depth - 2);
            let head = format!(
                r#"{{"jsonrpc":"2.0","id":2,"result":{{"tools":{},"nextCursor":null,"p":""#,
                serde_json::to_string(&granted()).unwrap()
            );
            let head = head.replace("\"tools\":", &format!("\"n\":{nested},\"tools\":"));
            let line = format!("{head}{}\"}}}}\n", "x".repeat(bytes - head.len() - 3));
            output.as_mut().unwrap().write_all(line.as_bytes()).ok();
            Then::Read
        })
    })
}

#[test]
fn a_response_over_its_bytes_or_depth_ends_the_session_and_one_at_both_bounds_passes() {
    let limit = Err(Fatal::Failed(Failure::Response));
    let cases = [
        ("at both bounds", sized(RESPONSE_MAX, 64), Ok(2)),
        ("a byte over", sized(RESPONSE_MAX + 1, 64), limit.clone()),
        ("a level deeper", sized(4096, 65), limit),
    ];
    for (case, script, ended) in cases {
        let (offered, _) = startup(script, Duration::from_secs(20));
        assert_eq!((case, offered.map(|tools| tools.len())), (case, ended));
    }
}

#[test]
fn no_traffic_extends_the_startup_deadline_and_a_server_that_takes_nothing_ends_it() {
    let chatty = mcp_but(|frame, output| {
        (frame["method"] == "initialize").then(|| {
            let progress = json!({"jsonrpc": "2.0", "method": "notifications/progress"});
            let until = Instant::now() + Duration::from_secs(10);
            while say(output, &progress) & (Instant::now() < until) {
                std::thread::sleep(Duration::from_millis(20));
            }
            Then::Read
        })
    });
    let silent = mcp_but(|frame, _| (frame["method"] == "initialize").then_some(Then::Read));
    for (case, script) in [("chatty", chatty), ("silent", silent)] {
        let begun = Instant::now();
        let (offered, _) = startup(script, Duration::from_millis(400));
        let taken = begun.elapsed();
        assert_eq!(
            (case, offered),
            (case, Err(Fatal::Failed(Failure::Timeout)))
        );
        let bounded = (taken >= Duration::from_millis(400)) & (taken < Duration::from_secs(2));
        assert!(bounded, "{case}: {taken:?}");
    }
    // A server whose input is closed before initialize takes nothing.
    let (input, output, server) = fake(Box::new(|_, _| Then::Close));
    let (mut input, mut output) = (input, output);
    let plan = plan();
    // Its thread closes its input only once it has read a frame.
    input.write_all(b"{}\n").unwrap();
    while !server.is_finished() {
        std::thread::sleep(Duration::from_millis(5));
    }
    let mut stderr = Drain::new(pipe().0, &[]);
    let started = started(
        (&mut input, &mut output),
        &plan,
        within(Duration::from_secs(5)),
        &mut stderr,
    );
    assert_eq!(started.err(), Some(Fatal::Failed(Failure::Protocol)));
}

/// The broker's ends of a harness: its input, written without blocking,
/// and its output, read as frames, within sixty seconds each: a liveness
/// bound that only sets how long a genuinely broken session takes to
/// fail. Twenty seconds was outrun by one eight-mebibyte frame when the
/// workspace suite ran on a loaded host.
struct Harness {
    to: Option<PipeWriter>,
    from: PipeReader,
    held: Vec<u8>,
}

impl Harness {
    fn send(&mut self, frame: &Value) {
        self.raw(format!("{frame}\n").as_bytes());
    }

    fn raw(&mut self, bytes: &[u8]) {
        let until = Instant::now() + Duration::from_secs(60);
        let to = self.to.as_mut().unwrap();
        assert!(offered(to, bytes, until), "the broker took nothing");
    }

    /// The broker's next frame to the harness, or none at its end.
    fn next(&mut self) -> Option<Value> {
        self.line()
            .map(|line| serde_json::from_slice(&line).unwrap())
    }

    /// The broker's next frame to the harness as its very bytes, its
    /// newline included, or none at its end.
    fn line(&mut self) -> Option<Vec<u8>> {
        let (until, mut searched) = (Instant::now() + Duration::from_secs(60), 0);
        loop {
            let newline = self.held[searched..].iter().position(|byte| *byte == b'\n');
            if let Some(at) = newline {
                return Some(self.held.drain(..=searched + at).collect());
            }
            searched = self.held.len();
            let mut chunk = [0; 1 << 16];
            match self.from.read(&mut chunk) {
                Ok(0) => return None,
                Ok(read) => self.held.extend_from_slice(&chunk[..read]),
                Err(error) if error.kind() == ErrorKind::WouldBlock => {
                    assert!(Instant::now() < until, "the broker answered nothing");
                    std::thread::sleep(TURN);
                }
                Err(error) => panic!("{error}"),
            }
        }
    }
}

/// What one served session left: how it ended, what its settled ledger
/// proves, when each of its records was made durable, each frame its
/// server read, the ledger's bytes, and the diagnostics it wrote.
struct Ran {
    ended: Result<(), Ended>,
    proved: Proved,
    durable: Vec<Instant>,
    frames: Vec<Value>,
    ledger: Vec<u8>,
    diagnostics: Vec<u8>,
}

/// What a test session's output edge holds: the plan it serves, the
/// secrets bound to it, what its server writes on its standard error
/// before ending it, how its ledger retains responses, if it does, and
/// whether a signal hung up on its harness (U9b1e).
struct Beside {
    plan: Plan,
    bindings: Vec<BoundSecret>,
    stderr: Vec<u8>,
    retain: Option<retention::Retain>,
    hung_up: &'static dyn Fn() -> bool,
}

impl Beside {
    /// The test plan, no secret bound, nothing on standard error, no signal.
    fn plain() -> Beside {
        Beside {
            plan: plan(),
            bindings: Vec::new(),
            stderr: Vec::new(),
            retain: None,
            hung_up: &live,
        }
    }
}

/// Each call's tool and outcome, the closure, and where verification
/// stopped.
type Proved = (
    Vec<(String, Option<Outcome>)>,
    Option<Disposition>,
    Option<Invalid>,
);

/// What a test's ledger made durable, and when, how many appends it was
/// asked for, and the one, counted from one, that fails with nothing of it
/// written.
#[derive(Default)]
struct Disk {
    bytes: Vec<u8>,
    at: Vec<Instant>,
    appends: usize,
    failing: Option<usize>,
}

/// The session served over `script`'s server with the harness speaking
/// `conversation` on a thread of its own, its ledger's append `failing`
/// failing, within `attempt`; settled as `serve` settles it.
fn session(
    script: Script,
    failing: Option<usize>,
    attempt: Startup<'static>,
    conversation: impl FnOnce(&mut Harness) + Send + 'static,
) -> Ran {
    bounded(script, failing, (attempt, None), conversation)
}

/// A [`session`] whose calls are each bound by `call`, where it is given,
/// or else by MB3's.
fn bounded(
    script: Script,
    failing: Option<usize>,
    bounds: (Startup<'static>, Option<Duration>),
    conversation: impl FnOnce(&mut Harness) + Send + 'static,
) -> Ran {
    edged(script, (failing, bounds), Beside::plain(), conversation)
}

/// A [`bounded`] session beside `beside`'s edge: its plan, bindings and
/// standard error, which a thread of its own writes and then ends.
fn edged(
    script: Script,
    (failing, (attempt, call)): (Option<usize>, (Startup<'static>, Option<Duration>)),
    Beside {
        plan,
        bindings,
        stderr,
        retain,
        hung_up,
    }: Beside,
    conversation: impl FnOnce(&mut Harness) + Send + 'static,
) -> Ran {
    let (mut input, mut output, server) = fake(script);
    let (errors, mut writes) = pipe();
    let writer = std::thread::spawn(move || writes.write_all(&stderr).ok());
    let mut drain = Drain::new(errors, &bindings);
    let disk = Rc::new(RefCell::new(Disk {
        failing,
        ..Disk::default()
    }));
    let durable = Rc::clone(&disk);
    let append = move |line: &[u8]| {
        let mut disk = durable.borrow_mut();
        disk.appends += 1;
        match disk.failing == Some(disk.appends) {
            true => Err(std::io::Error::other("fault")),
            false => {
                disk.bytes.extend_from_slice(line);
                disk.at.push(Instant::now());
                Ok(())
            }
        }
    };
    // Opened before the session's bindings are known, as `serve` opens it.
    let mut ledger = Ledger::open_with(append, (&plan.owner, DIGEST), &[]).unwrap();
    retain.into_iter().for_each(|retain| retain(&mut ledger));
    let ((asked, to), (from, answers)) = (pipe(), pipe());
    rustix::io::ioctl_fionbio(&from, true).unwrap();
    rustix::io::ioctl_fionbio(&to, true).unwrap();
    let talk = std::thread::spawn(move || {
        let mut harness = Harness {
            to: Some(to),
            from,
            held: Vec::new(),
        };
        conversation(&mut harness);
    });
    let harness = (asked, answers, hung_up);
    // Bound once its bindings are looked up, before its server starts, as
    // `serve`'s binder binds it; its standard error drained from its start,
    // and its tail kept once it ends, as `serving` drains and keeps it.
    let ended = ledger
        .bind(&bindings)
        .map_err(Ended::from)
        .and_then(|()| {
            let link = (&mut input, &mut output);
            started(link, &plan, attempt, &mut drain).map_err(Ended::Failed)
        })
        .and_then(|started| {
            let edge = Edge {
                bindings: &bindings,
                stderr: &mut drain,
            };
            match call {
                None => served(started, &plan, &mut ledger, harness, attempt, edge),
                Some(call) => served_within(
                    (call, std::thread::sleep),
                    started,
                    &plan,
                    &mut ledger,
                    harness,
                    attempt,
                    edge,
                ),
            }
        });
    let diagnostics: Vec<u8> = drain
        .tail()
        .iter()
        .flat_map(|said| said.as_str().bytes())
        .collect();
    let ended = settled(ledger, ended);
    drop((input, output));
    talk.join().unwrap();
    writer.join().unwrap();
    let frames = server.join().unwrap().0;
    let (lifecycle, invalid) = recover(&disk.borrow().bytes, &plan.owner, DIGEST);
    let proved = (
        lifecycle.calls().to_vec(),
        lifecycle.closed().cloned(),
        invalid,
    );
    let durable = disk.borrow().at.clone();
    let ledger = disk.borrow().bytes.clone();
    Ran {
        ended,
        proved,
        durable,
        frames,
        ledger,
        diagnostics,
    }
}

/// A served session the attempt gives ten seconds, its ledger whole.
fn conversed(script: Script, conversation: impl FnOnce(&mut Harness) + Send + 'static) -> Ran {
    session(script, None, within(Duration::from_secs(10)), conversation)
}

/// The harness's request `method` with `params`, as its request `id`.
fn request(id: Value, method: &str, params: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params})
}

/// The harness's call of `tool` with no arguments, as its request `id`.
fn call(id: &str, tool: &str) -> Value {
    request(
        json!(id),
        "tools/call",
        json!({"name": tool, "arguments": {}}),
    )
}

/// The server's frame for the broker's forwarding of a call of `tool`
/// with `arguments`, as the broker's request `id`.
fn forwarded(id: i64, tool: &str, arguments: Value) -> Value {
    request(
        json!(id),
        "tools/call",
        json!({"name": tool, "arguments": arguments}),
    )
}

/// What the well-behaved server's answer to a call of `tool` delivers to
/// the harness's request `id`.
fn echoed(id: &str, tool: &str) -> Value {
    result(
        &json!(id),
        json!({"content": [{"type": "text", "text": tool}]}),
    )
}

/// A ledger closed as `serve` closes a session served until the harness
/// ended, with `calls`.
fn admitted(calls: Vec<(&str, Option<Outcome>)>) -> Proved {
    (owned(calls), Some(Disposition::Clean), None)
}

/// A ledger closed Failed with `failure`, with `calls`.
fn failed(calls: Vec<(&str, Option<Outcome>)>, failure: Failure) -> Proved {
    (owned(calls), Some(Disposition::Failed(failure)), None)
}

/// Check that the session `ran`, labelled `case`, ended with `failure`
/// latched, its one call, of `lookup`, failed.
fn lookup_failed(case: &str, ran: Ran, failure: Failure) {
    let ended = Err(Ended::Failed(Fatal::Failed(failure.clone())));
    let proved = failed(vec![("lookup", Some(Outcome::Failed))], failure);
    assert_eq!((case, ran.ended, ran.proved), (case, ended, proved));
}

fn owned(calls: Vec<(&str, Option<Outcome>)>) -> Vec<(String, Option<Outcome>)> {
    calls
        .into_iter()
        .map(|(tool, outcome)| (tool.to_string(), outcome))
        .collect()
}

/// A harness that has said `frames` and was answered with `answers`, and
/// then that the broker said nothing more before the harness ended.
fn exchange(frames: Vec<Value>, answers: Vec<Value>) -> impl FnOnce(&mut Harness) + Send {
    move |harness| {
        for frame in &frames {
            harness.send(frame);
        }
        let heard: Vec<Value> = answers.iter().map(|_| harness.next().unwrap()).collect();
        assert_eq!(heard, answers);
        harness.send(&request(json!("last"), "ping", json!({})));
        assert_eq!(harness.next(), Some(result(&json!("last"), json!({}))));
    }
}

#[test]
fn the_harness_is_answered_here_and_its_granted_call_forwarded_once_recorded() {
    let greeting = json!({"protocolVersion": "2025-06-18", "capabilities": {"tools": {}},
                          "serverInfo": {"name": "cap-library-docs", "version": "1.4.2"}});
    let asked = vec![
        request(
            json!(0),
            "initialize",
            json!({"protocolVersion": "2025-06-18"}),
        ),
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
        request(json!("p"), "ping", json!({})),
        request(json!(1), "tools/list", json!({})),
        request(json!(2), "resources/list", json!({})),
        // MCP's negotiation: another version is answered with MB3's.
        request(
            json!(3),
            "initialize",
            json!({"protocolVersion": "2025-11-25"}),
        ),
        json!({"jsonrpc": "2.0", "id": 4, "method": "initialize"}),
        request(json!(5), "initialize", json!(["2025-06-18"])),
        request(json!(6), "initialize", json!({"protocolVersion": 7})),
        request(
            json!("c"),
            "tools/call",
            json!({"name": "lookup", "arguments": {"q": "x"}}),
        ),
    ];
    let answers = vec![
        result(&json!(0), greeting.clone()),
        result(&json!("p"), json!({})),
        result(&json!(1), json!({"tools": granted()})),
        refusal(
            json!(2),
            -32601,
            "MCP method is not admitted by this broker",
        ),
        result(&json!(3), greeting),
        refusal(json!(4), -32600, "MCP tool call request is invalid"),
        refusal(json!(5), -32600, "MCP tool call request is invalid"),
        refusal(json!(6), -32600, "MCP tool call request is invalid"),
        echoed("c", "lookup"),
    ];
    // The call, asked last, is answered once the server has answered it.
    let ran = conversed(mcp(), exchange(asked, answers));
    assert_eq!(ran.ended, Ok(()));
    let mut forwarded_once = initialization();
    forwarded_once.push(forwarded(3, "lookup", json!({"q": "x"})));
    assert_eq!(ran.frames, forwarded_once);
    assert_eq!(
        ran.proved,
        admitted(vec![("lookup", Some(Outcome::Succeeded))])
    );
}

#[test]
fn an_ungranted_call_is_recorded_refused_and_an_invalid_one_refused_unrecorded() {
    let named = format!("a{}", "._:/-".repeat(51));
    assert_eq!(named.len(), 256);
    let over = "t".repeat(257);
    let ungranted =
        |tool: &str| format!("tool '{tool}' is not granted to capability 'library-docs'");
    let limit = "MCP request exceeds the broker limit";
    let invalid = "MCP tool call request is invalid";
    let shapes = [
        json!({"name": "_lookup"}),
        json!({"name": "web search"}),
        json!({"name": ""}),
        json!({"name": "lookup", "arguments": []}),
        json!({"name": "lookup", "arguments": null}),
        json!({"name": "lookup", "_meta": 1}),
        json!({"name": "lookup", "_meta": null}),
        json!({"name": "lookup", "extra": 1}),
        json!({"tool": "lookup"}),
        json!(["lookup", {}]),
    ];
    let mut asked = vec![call("1", "admin"), call("2", &named), call("3", &over)];
    asked.extend(
        shapes
            .iter()
            .map(|params| request(json!("s"), "tools/call", params.clone())),
    );
    // A null id, and a frame by position: neither read, nor its id.
    let lookup = json!({"name": "lookup"});
    let frames = [
        request(Value::Null, "tools/call", lookup.clone()),
        json!(["2.0", "x", "tools/call", lookup]),
    ];
    asked.extend(frames);
    // Arguments left out are none, and metadata is never passed on.
    let meta = json!({"name": "lookup", "_meta": {"progressToken": 1}});
    asked.push(request(json!("m"), "tools/call", meta));
    let mut answers = vec![
        refusal(json!("1"), -32602, &ungranted("admin")),
        refusal(json!("2"), -32602, &ungranted(&named)),
        refusal(json!("3"), -32600, limit),
    ];
    answers.extend(shapes.iter().map(|_| refusal(json!("s"), -32600, invalid)));
    answers.extend([0, 1].map(|_| refusal(Value::Null, -32600, invalid)));
    answers.push(echoed("m", "lookup"));
    let ran = conversed(mcp(), exchange(asked, answers));
    assert_eq!(ran.ended, Ok(()));
    let mut forwarded_once = initialization();
    forwarded_once.push(forwarded(3, "lookup", json!({})));
    assert_eq!(ran.frames, forwarded_once);
    let refused = Some(Outcome::Refused(Denial::Ungranted));
    let calls = vec![
        ("admin", refused),
        (&named, refused),
        ("lookup", Some(Outcome::Succeeded)),
    ];
    assert_eq!(ran.proved, admitted(calls));
}

#[test]
fn requests_past_their_bounds_refuse_before_acceptance_and_ones_at_them_are_served() {
    let limit = |id: Value| refusal(id, -32600, "MCP request exceeds the broker limit");
    let padded = |bytes: usize| {
        let empty = call("at", "lookup").to_string().len();
        let pad = "p".repeat(bytes - empty - r#""x":"""#.len());
        request(
            json!("at"),
            "tools/call",
            json!({"name": "lookup", "arguments": {"x": pad}}),
        )
    };
    let nested = |depth: usize| {
        let deep = "[".repeat(depth - 3) + &"]".repeat(depth - 3);
        let arguments = format!(r#"{{"x":{deep}}}"#);
        let params = format!(r#"{{"name":"lookup","arguments":{arguments}}}"#);
        format!(r#"{{"jsonrpc":"2.0","id":"d","method":"tools/call","params":{params}}}"#)
    };
    let at = padded(REQUEST_MAX);
    assert_eq!(at.to_string().len(), REQUEST_MAX);
    let conversation = move |harness: &mut Harness| {
        // A byte over, and a frame over twice the bound: each refused once,
        // with no id, and the frame after it taken whole.
        for over in [REQUEST_MAX + 1, 2 * REQUEST_MAX + 7] {
            harness.raw(&[&vec![b'x'; over][..], b"\n"].concat());
            assert_eq!(harness.next(), Some(limit(Value::Null)));
        }
        harness.raw(format!("{}\n", nested(65)).as_bytes());
        assert_eq!(harness.next(), Some(limit(Value::Null)));
        harness.raw(b"{not json\n");
        let invalid = refusal(Value::Null, -32600, "MCP tool call request is invalid");
        assert_eq!(harness.next(), Some(invalid));
        harness.send(&at);
        assert_eq!(harness.next(), Some(echoed("at", "lookup")));
        harness.raw(format!("{}\n", nested(64)).as_bytes());
        assert_eq!(harness.next(), Some(echoed("d", "lookup")));
    };
    let ran = conversed(mcp(), conversation);
    assert_eq!(ran.ended, Ok(()));
    let succeeded = Some(Outcome::Succeeded);
    assert_eq!(
        ran.proved,
        admitted(vec![("lookup", succeeded), ("lookup", succeeded)])
    );
    assert_eq!(ran.frames.len(), 5);
}

/// A well-behaved server that answers each call only after `delay`.
fn slow(delay: Duration) -> Script {
    mcp_but(move |frame, _| {
        (frame["method"] == "tools/call").then(|| {
            std::thread::sleep(delay);
            None::<Then>
        })?
    })
}

#[test]
fn a_call_made_while_another_is_active_is_recorded_refused_and_never_forwarded() {
    let asked = vec![call("1", "lookup"), call("2", "read")];
    let answers = vec![
        refusal(json!("2"), -32600, "MCP broker already has an active call"),
        echoed("1", "lookup"),
    ];
    let ran = conversed(slow(Duration::from_millis(300)), exchange(asked, answers));
    assert_eq!(ran.ended, Ok(()));
    assert_eq!(ran.frames[3..], [forwarded(3, "lookup", json!({}))]);
    let active = Some(Outcome::Refused(Denial::Active));
    let expected = vec![("lookup", Some(Outcome::Succeeded)), ("read", active)];
    assert_eq!(ran.proved, admitted(expected));
}

#[test]
fn a_tools_error_is_its_calls_outcome_and_the_session_goes_on() {
    // The server answers its first call of `read` as a tool error, with
    // structured content and metadata, and its second with a JSON-RPC
    // error of its own, with data; each is passed on whole.
    let tool_error =
        json!({"content": [], "isError": true, "structuredContent": {"a": 1}, "_meta": {"k": 1}});
    let error = json!({"code": -32602, "message": "unknown", "data": {"why": [1]}});
    let (said, failed_with) = (tool_error.clone(), error.clone());
    let mut answering = 0;
    let failing_twice = mcp_but(move |frame, output| {
        let read = frame["params"]["name"] == "read";
        answering += usize::from(read);
        let answer = match answering {
            1 if read => result(&frame["id"], said.clone()),
            2 if read => json!({"jsonrpc": "2.0", "id": frame["id"], "error": failed_with.clone()}),
            _ => return None,
        };
        say(output, &answer);
        Some(Then::Read)
    });
    let asked = [call("1", "read"), call("2", "read"), call("3", "lookup")];
    let answers = vec![
        result(&json!("1"), tool_error),
        json!({"jsonrpc": "2.0", "id": "2", "error": error}),
        echoed("3", "lookup"),
    ];
    let conversation = move |harness: &mut Harness| {
        for (asked, answer) in asked.iter().zip(&answers) {
            harness.send(asked);
            assert_eq!(harness.next().as_ref(), Some(answer));
        }
    };
    let ran = conversed(failing_twice, conversation);
    assert_eq!(ran.ended, Ok(()));
    let failed_call = Some(Outcome::Failed);
    let calls = vec![
        ("read", failed_call),
        ("read", failed_call),
        ("lookup", Some(Outcome::Succeeded)),
    ];
    assert_eq!(ran.proved, admitted(calls));
}

/// A well-behaved server that answers its call by writing `said` instead,
/// and then whatever a well-behaved one answers.
fn answering(said: impl Fn(&Value) -> Vec<Value> + Send + 'static) -> Script {
    mcp_but(move |frame, output| {
        (frame["method"] == "tools/call").then(|| {
            for frame in said(frame) {
                say(output, &frame);
            }
            Then::Read
        })
    })
}

/// What the harness hears of a call `1` of `lookup` the session ends with
/// `cause`: the session's error, then the broker's end.
fn ended_with(cause: &'static str) -> impl FnOnce(&mut Harness) + Send {
    move |harness| {
        harness.send(&call("1", "lookup"));
        assert_eq!(harness.next(), Some(refusal(json!("1"), -32603, cause)));
        assert_eq!(harness.next(), None);
    }
}

#[test]
fn a_mismatched_or_late_response_ends_the_session_and_forwards_nothing() {
    let protocol = "MCP server protocol is invalid";
    let answered = |id: Value| move |_: &Value| vec![result(&id, json!({"content": []}))];
    let said = |frame: Value| answering(move |_| vec![frame.clone()]);
    let body = |body: Value| said(result(&json!(3), body));
    let error = |error: Value| said(json!({"jsonrpc": "2.0", "id": 3, "error": error}));
    let both = |result: Value, error: Value| {
        said(json!({"jsonrpc": "2.0", "id": 3, "result": result, "error": error}))
    };
    let cases: Vec<(&str, Script)> = vec![
        ("a string for a number", answering(answered(json!("3")))),
        ("an unknown id", answering(answered(json!(99)))),
        ("initialize's", answering(answered(json!(1)))),
        ("a null id", answering(answered(Value::Null))),
        // A body not in JSON-RPC's or MCP's shape is the server's fault,
        // never a tool's outcome.
        ("an error that is text", error(json!("oops"))),
        ("an error without a message", error(json!({"code": 1}))),
        (
            "an error with another member",
            error(json!({"code": 1, "message": "m", "x": 1})),
        ),
        ("an error by position", error(json!([1, "m"]))),
        ("a scalar result", body(json!(5))),
        ("a result without content", body(json!({}))),
        ("content not of blocks", body(json!({"content": [1]}))),
        (
            "a word on failure not a boolean",
            body(json!({"content": [], "isError": "yes"})),
        ),
        (
            "a null word on failure",
            body(json!({"content": [], "isError": null})),
        ),
        (
            "structured content not an object",
            body(json!({"content": [], "structuredContent": 1})),
        ),
        (
            "a null result beside an error",
            both(Value::Null, json!({"code": 1, "message": "m"})),
        ),
        (
            "a null error beside a result",
            both(json!({"content": []}), Value::Null),
        ),
    ];
    for (case, script) in cases {
        lookup_failed(
            case,
            conversed(script, ended_with(protocol)),
            Failure::Protocol,
        );
    }
    // A second answer to a call already answered: the first delivered, the
    // second ends the session, whose cause is still its first.
    let twice = answering(|frame| vec![answer(frame).unwrap(), answer(frame).unwrap()]);
    let conversation = |harness: &mut Harness| {
        harness.send(&call("1", "lookup"));
        assert_eq!(harness.next(), Some(echoed("1", "lookup")));
        assert_eq!(harness.next(), None);
    };
    let ran = conversed(twice, conversation);
    assert_eq!(
        ran.ended,
        Err(Ended::Failed(Fatal::Failed(Failure::Protocol)))
    );
    let succeeded = vec![("lookup", Some(Outcome::Succeeded))];
    assert_eq!(ran.proved, failed(succeeded, Failure::Protocol));
}

/// A server that answers a call with nothing but `frame`, every 20
/// milliseconds, until the broker stops reading.
fn flooding(frame: Value) -> Script {
    mcp_but(move |asked, output| {
        (asked["method"] == "tools/call").then(|| {
            let until = Instant::now() + Duration::from_secs(10);
            while say(output, &frame) & (Instant::now() < until) {
                std::thread::sleep(Duration::from_millis(20));
            }
            Then::Read
        })
    })
}

#[test]
fn no_traffic_extends_a_calls_deadline_and_none_spends_past_its_budget() {
    // Progress and the server's pings, each answered, never the call: the
    // attempt's deadline ends it, and the session.
    for frame in [
        json!({"jsonrpc": "2.0", "method": "notifications/progress", "params": {"p": 1}}),
        json!({"jsonrpc": "2.0", "id": "ping", "method": "ping"}),
    ] {
        let begun = Instant::now();
        let ran = session(
            flooding(frame.clone()),
            None,
            within(Duration::from_millis(800)),
            ended_with("MCP server call timed out"),
        );
        let taken = begun.elapsed();
        lookup_failed("flooded", ran, Failure::Timeout);
        assert!(taken < Duration::from_secs(3), "{taken:?}");
    }
    // Frames that answer nothing, past the operation's budget.
    let pad = "x".repeat(64 << 10);
    let notice = json!({"jsonrpc": "2.0", "method": "notifications/message", "params": {"p": pad}});
    let ran = conversed(
        flooding(notice),
        ended_with("MCP server response exceeds the broker limit"),
    );
    lookup_failed("over budget", ran, Failure::Response);
    // The listing spends its whole budget, and the call its own whole one.
    let spending = mcp_but(|frame, output| {
        (frame["method"] != "notifications/initialized").then(|| {
            (frame["method"] != "initialize").then(|| noise(output, NOTICES_MAX));
            say(output, &answer(frame).unwrap());
            Then::Read
        })
    });
    let asked = vec![call("1", "lookup")];
    let ran = conversed(spending, exchange(asked, vec![echoed("1", "lookup")]));
    assert_eq!(ran.ended, Ok(()));
    let succeeded = vec![("lookup", Some(Outcome::Succeeded))];
    assert_eq!(ran.proved, admitted(succeeded));
}

/// The harness's cancellation of its request `id`.
fn cancel(id: &str) -> Value {
    let params = json!({"requestId": id, "reason": "user"});
    json!({"jsonrpc": "2.0", "method": "notifications/cancelled", "params": params})
}

/// The server's frame for the broker's cancellation of its request `id`.
fn passed(id: i64) -> Value {
    json!({"jsonrpc": "2.0", "method": "notifications/cancelled", "params": {"requestId": id}})
}

#[test]
fn a_cancelled_call_is_passed_on_and_never_delivered() {
    let ping = request(json!("p"), "ping", json!({}));
    // Settled at once: recorded, not delivered; the session goes on. A
    // cancellation of no active call, one without a request, or one by
    // position, is nothing.
    let conversation = move |harness: &mut Harness| {
        harness.send(&cancel("none"));
        harness.send(&json!({"jsonrpc": "2.0", "method": "notifications/cancelled"}));
        harness.send(&call("0", "lookup"));
        let positional =
            json!({"jsonrpc": "2.0", "method": "notifications/cancelled", "params": ["0"]});
        harness.send(&positional);
        assert_eq!(harness.next(), Some(echoed("0", "lookup")));
        harness.send(&call("1", "lookup"));
        harness.send(&cancel("1"));
        std::thread::sleep(Duration::from_millis(300));
        harness.send(&ping);
        assert_eq!(harness.next(), Some(result(&json!("p"), json!({}))));
    };
    let ran = conversed(slow(Duration::from_millis(100)), conversation);
    assert_eq!(ran.ended, Ok(()));
    let calls = [
        forwarded(3, "lookup", json!({})),
        forwarded(4, "lookup", json!({})),
    ];
    assert_eq!(
        ran.frames[3..],
        [calls[0].clone(), calls[1].clone(), passed(4)]
    );
    let succeeded = Some(Outcome::Succeeded);
    assert_eq!(
        ran.proved,
        admitted(vec![("lookup", succeeded), ("lookup", succeeded)])
    );
    // Never settled: two seconds on, the call and the session end, and the
    // harness, which cancelled it, hears nothing of it. Cancelled again, it
    // is passed on once.
    let never = mcp_but(|frame, _| (frame["method"] == "tools/call").then_some(Then::Read));
    let begun = Instant::now();
    let conversation = move |harness: &mut Harness| {
        harness.send(&call("1", "lookup"));
        harness.send(&cancel("1"));
        harness.send(&cancel("1"));
        assert_eq!(harness.next(), None);
    };
    let ran = conversed(never, conversation);
    let taken = begun.elapsed();
    assert_eq!(
        ran.frames[3..],
        [forwarded(3, "lookup", json!({})), passed(3)]
    );
    lookup_failed("never settled", ran, Failure::Timeout);
    let settled_in = (taken >= Duration::from_secs(2)) & (taken < Duration::from_secs(4));
    assert!(settled_in, "{taken:?}");
}

/// A well-behaved server that does `then` with its input once it has
/// listed its tools.
fn listing_then(then: Then) -> Script {
    mcp_but(move |frame, output| {
        (frame["method"] == "tools/list").then(|| {
            say(output, &answer(frame).unwrap());
            then.clone()
        })
    })
}

/// The harness's call `1` of `lookup`, larger than any pipe holds.
fn large() -> Value {
    let arguments = json!({"x": "y".repeat(1 << 18)});
    request(
        json!("1"),
        "tools/call",
        json!({"name": "lookup", "arguments": arguments}),
    )
}

#[test]
fn a_server_that_takes_no_call_ends_it() {
    // Its input closed before it has listed, its output ended after: the
    // forwarding fails, or, taken by a pipe anyway, is ended ([`Then::Gone`]).
    let gone = mcp_but(|frame, _| {
        (frame["method"] == "tools/list").then(|| Then::Gone(answer(frame).unwrap()))
    });
    let ran = conversed(gone, ended_with("MCP server protocol is invalid"));
    lookup_failed("closing", ran, Failure::Protocol);
    // Its input held unread once it has listed: a call larger than its
    // pipe holds is not taken by the attempt's deadline.
    let conversation = move |harness: &mut Harness| {
        harness.send(&large());
        let timed_out = refusal(json!("1"), -32603, "MCP server call timed out");
        assert_eq!(harness.next(), Some(timed_out));
    };
    let ran = session(
        listing_then(Then::Hold),
        None,
        within(Duration::from_millis(800)),
        conversation,
    );
    lookup_failed("holding", ran, Failure::Timeout);
}

#[test]
fn a_servers_requests_during_a_call_are_answered_by_the_broker_alone() {
    let asking = answering(|frame| {
        vec![
            json!({"jsonrpc": "2.0", "id": "p", "method": "ping"}),
            json!({"jsonrpc": "2.0", "id": "r", "method": "roots/list"}),
            answer(frame).unwrap(),
        ]
    });
    let ran = conversed(
        asking,
        exchange(vec![call("1", "lookup")], vec![echoed("1", "lookup")]),
    );
    assert_eq!(ran.ended, Ok(()));
    let method = "MCP method is not admitted by this broker";
    let answered = [
        result(&json!("p"), json!({})),
        refusal(json!("r"), -32601, method),
    ];
    assert_eq!(ran.frames[4..], answered);
    assert_eq!(
        ran.proved,
        admitted(vec![("lookup", Some(Outcome::Succeeded))])
    );
}

#[test]
fn no_call_is_forwarded_or_delivered_unrecorded() {
    let unrecorded = "broker call could not be recorded";
    let unrecordable = Err(Ended::Unrecorded(Unrecorded::Record));
    let unclosed = |calls| (owned(calls), None, Some(Invalid::Lifecycle));
    // The Opened is the first append: a Started not made is acted on
    // nowhere, and records nothing after it.
    let ten = || within(Duration::from_secs(10));
    let ran = session(mcp(), Some(2), ten(), ended_with(unrecorded));
    assert_eq!(ran.ended, unrecordable);
    assert_eq!(ran.frames, initialization());
    assert_eq!(ran.proved, unclosed(vec![]));
    // A Terminal not made: forwarded, but its answer never delivered.
    let ran = session(mcp(), Some(3), ten(), ended_with(unrecorded));
    assert_eq!(ran.ended, unrecordable);
    assert_eq!(ran.frames.len(), 4);
    assert_eq!(ran.proved, unclosed(vec![("lookup", None)]));
    // A refused Terminal not made: its refusal never delivered.
    let refusing = |harness: &mut Harness| {
        harness.send(&call("1", "admin"));
        let unrecorded = refusal(json!("1"), -32603, "broker call could not be recorded");
        assert_eq!(harness.next(), Some(unrecorded));
        assert_eq!(harness.next(), None);
    };
    let ran = session(mcp(), Some(3), ten(), refusing);
    assert_eq!(ran.ended, unrecordable);
    assert_eq!(ran.proved, unclosed(vec![("admin", None)]));
    // A fatal ending whose Terminal is not made: the harness is told its
    // call could not be recorded, and the session's cause is still its
    // own.
    let mismatched = answering(|_| vec![result(&json!("3"), json!({}))]);
    let ran = session(mismatched, Some(3), ten(), ended_with(unrecorded));
    assert_eq!(
        ran.ended,
        Err(Ended::Failed(Fatal::Failed(Failure::Protocol)))
    );
    assert_eq!(ran.proved, unclosed(vec![("lookup", None)]));
}

/// A harness that makes 4,095 calls of `admin`, then the 4,096th of
/// `lookup` and `after` it, and has heard `heard` by its end.
fn at_the_bound(after: Vec<Value>, heard: Vec<Value>) -> impl FnOnce(&mut Harness) + Send {
    move |harness| {
        let mut to = harness.to.take().unwrap();
        let writer = std::thread::spawn(move || {
            let until = Instant::now() + Duration::from_secs(20);
            let admin = (0..4095).map(|at| call(&at.to_string(), "admin"));
            let last = std::iter::once(call("last", "lookup")).chain(after);
            for frame in admin.chain(last) {
                offered(&mut to, format!("{frame}\n").as_bytes(), until);
            }
            to
        });
        let said: Vec<Value> = std::iter::from_fn(|| harness.next()).collect();
        assert_eq!(
            (said.len(), &said[4095..]),
            (4095 + heard.len(), &heard[..])
        );
        drop(writer.join().unwrap());
    }
}

#[test]
fn at_the_call_bound_every_accepted_call_is_kept_and_the_harness_still_read() {
    // 4,095 calls refused here, then a 4,096th forwarded: once the bound is
    // reached a ping is still answered, and the active call delivered
    // before the session ends; a call made then ends it with the limit,
    // failing the active call, of which the harness is told.
    let ping = request(json!("p"), "ping", json!({}));
    let pong = result(&json!("p"), json!({}));
    let limit = refusal(
        json!("last"),
        -32603,
        "broker ledger exceeds the attempt limit",
    );
    let cases = [
        (
            vec![ping.clone()],
            vec![pong.clone(), echoed("last", "lookup")],
        ),
        (vec![ping, call("next", "lookup")], vec![pong, limit]),
    ];
    for ((after, heard), outcome) in cases.into_iter().zip([Outcome::Succeeded, Outcome::Failed]) {
        let ran = conversed(slow(Duration::from_secs(2)), at_the_bound(after, heard));
        assert_eq!(ran.ended, Err(Ended::Failed(Fatal::Failed(Failure::Limit))));
        let (calls, closed, invalid) = ran.proved;
        let refused = (
            String::from("admin"),
            Some(Outcome::Refused(Denial::Ungranted)),
        );
        let mut kept = vec![refused; 4095];
        kept.push((String::from("lookup"), Some(outcome)));
        assert_eq!(calls, kept);
        assert_eq!(
            (closed, invalid),
            (Some(Disposition::Failed(Failure::Limit)), None)
        );
    }
}

#[test]
fn a_pipe_is_read_once_a_turn_and_frames_held_past_an_over_one_are_read_on() {
    let (reader, mut writer) = pipe();
    let mut lines = Lines::new(reader);
    // Bytes but no whole frame: one read, then its caller's turn, which
    // reads its deadline again; then nothing, which is quiet.
    writer.write_all(b"xxxx").unwrap();
    assert_eq!(lines.next(8), Line::Partial);
    assert_eq!(lines.next(8), Line::Quiet);
    // Past the bound: over as soon as it is known to be. Its end, two
    // whole frames and the pipe's end, in one write: the frames held are
    // read before the end.
    writer.write_all(b"xxxxxxxx").unwrap();
    assert_eq!(lines.next(8), Line::Over);
    writer.write_all(b"xx\n{a}\n{b}\n").unwrap();
    drop(writer);
    assert_eq!(lines.next(8), Line::Frame(b"{a}".to_vec()));
    assert_eq!(lines.next(8), Line::Frame(b"{b}".to_vec()));
    assert_eq!(lines.next(8), Line::Ended);
}

/// An attempt `within` from now, cancelled once the flag it gives is set.
fn cancellable(within: Duration) -> (Startup<'static>, &'static AtomicBool) {
    let flag: &'static AtomicBool = Box::leak(Box::new(AtomicBool::new(false)));
    let cancelled: &'static (dyn Fn() -> bool + Sync) =
        Box::leak(Box::new(move || flag.load(Ordering::SeqCst)));
    ((Instant::now() + within, cancelled), flag)
}

/// A server that answers no call.
fn silent() -> Script {
    mcp_but(|frame, _| (frame["method"] == "tools/call").then_some(Then::Read))
}

/// Check that the session `ran`, labelled `case`, timed out with no call
/// made, `begun` less than three seconds before.
fn idle_timeout(case: &str, ran: Ran, begun: Instant) {
    let ended = Err(Ended::Failed(Fatal::Failed(Failure::Timeout)));
    let proved = failed(vec![], Failure::Timeout);
    assert_eq!((case, ran.ended, ran.proved), (case, ended, proved));
    let taken = begun.elapsed();
    assert!(taken < Duration::from_secs(3), "{case}: {taken:?}");
}

#[test]
fn the_attempts_deadline_and_cancellation_end_an_idle_a_waiting_or_a_flooded_session() {
    // Idle: no request at all, and the attempt's deadline ends it.
    let begun = Instant::now();
    let ran = session(mcp(), None, within(Duration::from_millis(400)), |harness| {
        assert_eq!(harness.next(), None);
    });
    assert!(begun.elapsed() >= Duration::from_millis(400));
    idle_timeout("idle", ran, begun);
    // Cancelled while its call waits on a silent server: the call fails at
    // once, and the harness, whose starter has gone, is told nothing.
    let (attempt, flag) = cancellable(Duration::from_secs(10));
    let begun = Instant::now();
    let ran = session(silent(), None, attempt, move |harness| {
        harness.send(&call("1", "lookup"));
        std::thread::sleep(Duration::from_millis(300));
        flag.store(true, Ordering::SeqCst);
        assert_eq!(harness.next(), None);
    });
    lookup_failed("cancelled", ran, Failure::Timeout);
    assert!(begun.elapsed() < Duration::from_secs(3));
    // A frame over the bound that never ends, written as fast as it is
    // read: refused once, and the attempt's deadline still ends it.
    let begun = Instant::now();
    let ran = session(mcp(), None, within(Duration::from_millis(800)), |harness| {
        let (chunk, mut to) = (vec![b'x'; 64 << 10], harness.to.take().unwrap());
        let until = Instant::now() + Duration::from_secs(10);
        while offered(&mut to, &chunk, until) {}
        let limit = refusal(Value::Null, -32600, "MCP request exceeds the broker limit");
        assert_eq!(harness.next(), Some(limit));
        assert_eq!(harness.next(), None);
    });
    idle_timeout("flooded", ran, begun);
}

#[test]
fn a_harness_that_takes_no_answer_or_closes_its_output_ends_the_session() {
    // It asks a call of a silent server, then pings without reading its
    // answers, which fill its pipe: no write outlasts the call's deadline,
    // here the attempt's, and the call fails with the session.
    let begun = Instant::now();
    let ran = session(
        silent(),
        None,
        within(Duration::from_millis(800)),
        |harness| {
            harness.send(&call("1", "lookup"));
            let ping = format!("{}\n", request(json!("p"), "ping", json!({})));
            let (mut to, until) = (
                harness.to.take().unwrap(),
                Instant::now() + Duration::from_secs(10),
            );
            for _ in 0..10_000 {
                if !offered(&mut to, ping.as_bytes(), until) {
                    break;
                }
            }
        },
    );
    lookup_failed("stalled", ran, Failure::Timeout);
    assert!(begun.elapsed() < Duration::from_secs(3));
    // It closes its output, then asks a ping and a call: the ping's answer
    // finds no reader, which ends the session as the harness's end does,
    // and the call is never read.
    let ran = conversed(mcp(), |harness| {
        drop(std::mem::replace(&mut harness.from, pipe().0));
        harness.send(&request(json!("p"), "ping", json!({})));
        harness.send(&call("1", "lookup"));
    });
    assert_eq!(ran.ended, Ok(()));
    assert_eq!(ran.frames, initialization());
    assert_eq!(ran.proved, admitted(vec![]));
}

/// A server that lists `tools` and answers every call with `body`.
fn answering_with(tools: Vec<Value>, body: Value) -> Script {
    mcp_but(move |frame, output| {
        let said = match frame["method"].as_str() {
            Some("tools/list") => json!({"tools": tools}),
            Some("tools/call") => body.clone(),
            _ => return None,
        };
        say(output, &result(&frame["id"], said));
        Some(Then::Read)
    })
}

#[test]
fn an_answer_that_with_the_callers_own_id_passes_mb3s_bound_ends_the_session() {
    // A listing and a call's result each just within the bound, and the
    // harness's ids sized to bring each answer to exactly the bound, then
    // one byte past it.
    let fixed = r#"{"jsonrpc":"2.0","id":,"result":}"#.len();
    let id = |bytes: usize, over: usize| json!("i".repeat(RESPONSE_MAX - fixed - bytes - 2 + over));
    let empty = json!({"tools": [described("lookup", 0), described("read", 0)]});
    let catalog = RESPONSE_MAX - (64 << 10);
    let pad = catalog - empty.to_string().len();
    let tools = vec![described("lookup", pad), described("read", 0)];
    let text = |bytes: usize| json!({"content": [{"type": "text", "text": "t".repeat(bytes)}]});
    let size = RESPONSE_MAX - (32 << 10);
    let body = text(size - text(0).to_string().len());
    let lookup = |id: Value| request(id, "tools/call", json!({"name": "lookup", "arguments": {}}));
    let limit = |id: Value| refusal(id, -32603, "MCP server response exceeds the broker limit");
    let (list_at, call_at) = (id(catalog, 0), id(size, 0));
    let (list_over, call_over) = (id(catalog, 1), id(size, 1));
    let (listed, answered) = (json!({"tools": tools.clone()}), body.clone());
    let conversation = move |harness: &mut Harness| {
        harness.send(&request(list_at.clone(), "tools/list", json!({})));
        assert_eq!(harness.next(), Some(result(&list_at, listed)));
        harness.send(&lookup(call_at.clone()));
        assert_eq!(harness.next(), Some(result(&call_at, answered)));
        harness.send(&request(list_over.clone(), "tools/list", json!({})));
        assert_eq!(harness.next(), Some(limit(list_over)));
        assert_eq!(harness.next(), None);
    };
    // Four frames at the bound pass through the masker and the canonical
    // writer. Under coverage instrumentation that outlasts `conversed`'s ten
    // seconds even on a hosted runner, and the session then ends at its
    // deadline before the refusal is written. The bound is what this proves,
    // not the time, so each session here is given a minute.
    let patient = || within(Duration::from_secs(60));
    let answering = answering_with(tools.clone(), body.clone());
    let ran = session(answering, None, patient(), conversation);
    let succeeded = vec![("lookup", Some(Outcome::Succeeded))];
    let ended = Err(Ended::Failed(Fatal::Failed(Failure::Response)));
    assert_eq!(ran.ended, ended);
    assert_eq!(ran.proved, failed(succeeded, Failure::Response));
    let ran = session(
        answering_with(tools, body),
        None,
        patient(),
        move |harness| {
            harness.send(&lookup(call_over.clone()));
            assert_eq!(harness.next(), Some(limit(call_over)));
            assert_eq!(harness.next(), None);
        },
    );
    lookup_failed("a call's answer", ran, Failure::Response);
}

#[test]
fn a_call_the_server_does_not_take_keeps_neither_the_harness_nor_its_cancellation_waiting() {
    // The server takes nothing once it has listed, and the call is larger
    // than its pipe holds: the harness's cancellation is still read, and
    // two seconds on the call ends, long before the attempt's deadline.
    let begun = Instant::now();
    let ran = session(
        listing_then(Then::Hold),
        None,
        within(Duration::from_secs(20)),
        |harness| {
            harness.send(&large());
            harness.send(&cancel("1"));
            assert_eq!(harness.next(), None);
        },
    );
    let taken = begun.elapsed();
    lookup_failed("held", ran, Failure::Timeout);
    let settled_in = (taken >= Duration::from_secs(2)) & (taken < Duration::from_secs(5));
    assert!(settled_in, "{taken:?}");
}

/// A harness that asks `asked`, then neither reads nor says anything more,
/// its pipes kept open by `held` until the test drops them once the
/// session has ended.
fn unread(
    asked: Value,
    held: Sender<(PipeWriter, PipeReader)>,
) -> impl FnOnce(&mut Harness) + Send {
    move |harness| {
        harness.send(&asked);
        let from = std::mem::replace(&mut harness.from, pipe().0);
        held.send((harness.to.take().unwrap(), from)).unwrap();
    }
}

#[test]
fn an_answer_or_a_last_frame_the_harness_leaves_unread_waits_no_longer_than_its_bound() {
    // The harness's id is larger than any pipe holds, so no answer to it is
    // taken whole while the harness reads nothing.
    let id = json!("i".repeat(256 << 10));
    let lookup = request(id, "tools/call", json!({"name": "lookup", "arguments": {}}));
    // Answered at once: its answer waits for the call's own deadline, here
    // one second, never for MB3's thirty on an answer.
    let (held, holding) = channel();
    let begun = Instant::now();
    let attempt = (
        within(Duration::from_secs(20)),
        Some(Duration::from_secs(1)),
    );
    let ran = bounded(mcp(), None, attempt, unread(lookup.clone(), held));
    let taken = begun.elapsed();
    drop(holding);
    let succeeded = vec![("lookup", Some(Outcome::Succeeded))];
    let timed_out = Err(Ended::Failed(Fatal::Failed(Failure::Timeout)));
    assert_eq!(ran.ended, timed_out);
    assert_eq!(ran.proved, failed(succeeded, Failure::Timeout));
    assert!(taken < Duration::from_secs(5), "{taken:?}");
    // Answered by another id: the session's last frame, telling the call
    // why it failed, waits one second, and nothing more.
    let (held, holding) = channel();
    let begun = Instant::now();
    let mismatched = answering(|_| vec![result(&json!("3"), json!({}))]);
    let attempt = within(Duration::from_secs(20));
    let ran = session(mismatched, None, attempt, unread(lookup, held));
    let taken = begun.elapsed();
    drop(holding);
    lookup_failed("mismatched", ran, Failure::Protocol);
    assert!(taken < Duration::from_secs(5), "{taken:?}");
}

#[test]
fn a_cancellation_the_servers_ended_input_cannot_take_ends_the_session_with_its_first_cause() {
    // The server ends its input once it has read the call, its output
    // left silent, or answering the call after the cancellation.
    let silent = mcp_but(|frame, _| (frame["method"] == "tools/call").then_some(Then::Close));
    let answering_late = mcp_but(|frame, output| {
        (frame["method"] == "tools/call").then(|| {
            let (answer, mut output) = (answer(frame).unwrap(), output.take());
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(600));
                say(&mut output, &answer);
            });
            Then::Close
        })
    });
    for (case, script) in [("silent", silent), ("answering", answering_late)] {
        let begun = Instant::now();
        let ran = conversed(script, |harness| {
            harness.send(&call("1", "lookup"));
            std::thread::sleep(Duration::from_millis(300));
            harness.send(&cancel("1"));
            assert_eq!(harness.next(), None);
        });
        let taken = begun.elapsed();
        lookup_failed(case, ran, Failure::Protocol);
        assert!(taken < Duration::from_secs(2), "{case}: {taken:?}");
    }
}

/// The harness's ping, its id a string of `bytes` bytes, and that id.
fn ping_sized(bytes: usize) -> (Value, Value) {
    let id = json!("i".repeat(bytes));
    (request(id.clone(), "ping", json!({})), id)
}

/// A harness that asks a silent server's call `1` of `lookup`, then
/// `pings` without reading their answers, then cancels the call, holding
/// its pipes, with when it began its cancellation, until the test drops
/// them once the session has ended.
fn cancelled_behind(
    pings: Vec<Value>,
    held: Sender<(Instant, PipeWriter, PipeReader)>,
) -> impl FnOnce(&mut Harness) + Send {
    move |harness| {
        harness.send(&call("1", "lookup"));
        for ping in &pings {
            harness.send(ping);
        }
        let cancelled = Instant::now();
        harness.send(&cancel("1"));
        let from = std::mem::replace(&mut harness.from, pipe().0);
        held.send((cancelled, harness.to.take().unwrap(), from))
            .unwrap();
    }
}

#[test]
fn a_cancellation_behind_an_answer_the_harness_leaves_untaken_settles_in_two_seconds() {
    // A silent call is active; the harness pings with an id larger than its
    // output pipe holds, reads nothing, then cancels the call. The call's
    // own deadline, ten seconds, and the attempt's, thirty, are far off:
    // only the cancellation's two seconds can settle it this soon.
    let (ping, id) = ping_sized(512 << 10);
    let (held, holding) = channel();
    let attempt = (
        within(Duration::from_secs(30)),
        Some(Duration::from_secs(10)),
    );
    let ran = bounded(silent(), None, attempt, cancelled_behind(vec![ping], held));
    let ended = Instant::now();
    let (cancelled, to, mut from) = holding.recv().unwrap();
    // The Terminal, the third record, is durable two seconds after the
    // cancellation, give or take one second's scheduling on a loaded
    // two-CPU host; the session ends within CLOSING's one second more.
    let settled = ran.durable[2] - cancelled;
    let in_time = (settled >= Duration::from_secs(2)) & (settled < Duration::from_secs(3));
    assert!(in_time, "settled in {settled:?}");
    let taken = ended - cancelled;
    assert!(taken < Duration::from_secs(4), "ended in {taken:?}");
    // One call forwarded, and its cancellation, and no retry.
    let sent = [forwarded(3, "lookup", json!({})), passed(3)];
    assert_eq!(ran.frames[3..], sent);
    lookup_failed("untaken", ran, Failure::Timeout);
    // The harness's output holds a part of the ping's answer, never whole,
    // and nothing after it: no cancelled response is delivered.
    drop(to);
    let mut left = Vec::new();
    from.read_to_end(&mut left).unwrap();
    let whole = format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":{{}}}}\n");
    let blocked = (left.is_empty(), left.len() < whole.len());
    assert_eq!(blocked, (false, true));
    assert_eq!(left, whole.as_bytes()[..left.len()]);
}
