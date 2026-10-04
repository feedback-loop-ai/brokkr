//! Issue #532's acceptance, held by a test rather than by a grep in review:
//! the repository is public, so no tracked file names a private host or
//! address. The operator's Spark endpoints were once written into a guide
//! and a decision, and are words now; this keeps them out. The whole
//! tracked tree is read, as `hosts.rs` reads it, so the check is the same in
//! a pull request, in the merge queue and on `main`.
//!
//! A private host is spelled one of three ways:
//! - a URL, other than a `file://` one, whose host is a single label, such
//!   as an endpoint pasted from a route's `baseURL`; the names in
//!   [`PLACEHOLDERS`] name no machine;
//! - an IPv4 address in a private, shared (carrier-grade NAT, which tailnets
//!   number from) or link-local range, in a URL or standing alone;
//! - a name under one of [`SUFFIXES`], in a URL or standing alone, except
//!   [`URL_ONLY`], which is read in a URL only.
//!
//! Each file is read as `support/tracked_text.rs` reads it, so one that
//! cannot be read fails the check.
//!
//! Deliberately unread, each for the reason given:
//! - A Rust test file under `crates/`, one in a crate's `tests/` or named
//!   `tests.rs` or `*_tests.rs`, this checker among them: its endpoints are
//!   test vectors for the endpoint grammar (`https://-host/x`,
//!   `https://leaked:pw@host/x`). Production Rust source is read, as the
//!   place a pasted endpoint would ship from.
//! - A bare `name:port` outside a URL: `file:line` citations and clock times
//!   spell the same shape throughout the prose.
//! - An IPv6 literal: the slip this guards is the one #532 removed, a name
//!   or IPv4 address pasted from an endpoint.

use std::net::Ipv4Addr;
use std::path::Path;

#[path = "support/tracked.rs"]
mod tracked_files;
#[path = "support/tracked_text.rs"]
mod tracked_text;

/// Single-label hosts that name no machine: loopback, and the endpoint
/// grammar's own vectors as the #226 acceptance ledger quotes them.
const PLACEHOLDERS: [&str; 3] = ["localhost", "host", "hóst"];

/// Suffixes for names on a private network: RFC 6762's `.local`, RFC 8375's
/// `.home.arpa`, ICANN's `.internal`, and the router and tailnet names an
/// endpoint copied from a host carries.
const SUFFIXES: [&str; 6] = [
    "local",
    "localdomain",
    "lan",
    "internal",
    "home.arpa",
    "ts.net",
];

/// The suffix read inside a URL only: prose and data cite field paths such
/// as `expected.local`.
const URL_ONLY: &str = "local";

/// Whether `c` belongs to a host name or address.
fn host_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '-' | '.' | '_')
}

/// Why the IPv4 address `ip` is private, if it is.
fn private_ipv4(ip: Ipv4Addr) -> Option<&'static str> {
    let [first, second, ..] = ip.octets();
    if ip.is_private() {
        Some("a private address")
    } else if ip.is_link_local() {
        Some("a link-local address")
    } else if first == 100 && (64..128).contains(&second) {
        Some("a shared address")
    } else {
        None
    }
}

/// Why `name`, read inside a URL when `in_url`, names a private host or
/// address, if it does.
fn private(name: &str, in_url: bool) -> Option<&'static str> {
    let name = name.trim_end_matches('.').to_lowercase();
    if let Ok(ip) = name.parse::<Ipv4Addr>() {
        return private_ipv4(ip);
    }
    if !name.contains('.') {
        return (in_url && !PLACEHOLDERS.contains(&name.as_str())).then_some("a single-label host");
    }
    SUFFIXES
        .into_iter()
        .filter(|suffix| in_url || *suffix != URL_ONLY)
        .find(|suffix| {
            name.strip_suffix(suffix)
                .and_then(|rest| rest.strip_suffix('.'))
                .is_some_and(|label| !label.is_empty())
        })
        .map(|_| "under a private-use suffix")
}

/// The host of each URL on `line` that has one, as written.
fn url_hosts(line: &str) -> Vec<&str> {
    line.match_indices("://")
        .filter_map(|(at, _)| {
            let before = &line[..at];
            let scheme_len = before
                .chars()
                .rev()
                .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-'))
                .count();
            let scheme = &before[before.len() - scheme_len..];
            if scheme.is_empty() || scheme.eq_ignore_ascii_case("file") {
                return None;
            }
            let rest = &line[at + 3..];
            let end = rest
                .find(|c: char| c.is_whitespace() || "/?#\"'`()[]<>,\\".contains(c))
                .unwrap_or(rest.len());
            let authority = &rest[..end];
            let host = authority
                .rsplit_once('@')
                .map_or(authority, |(_, host)| host);
            let host = &host[..host.find(|c: char| !host_char(c)).unwrap_or(host.len())];
            (!host.is_empty()).then_some(host)
        })
        .collect()
}

/// Whether `path` is a Rust test file under `crates/`, whose endpoints are
/// the endpoint grammar's test vectors.
fn rust_test_file(path: &str) -> bool {
    let Some(rest) = path.strip_prefix("crates/") else {
        return false;
    };
    let in_tests_dir = rest
        .split_once('/')
        .is_some_and(|(_, inner)| inner.starts_with("tests/"));
    let name = rest.rsplit('/').next().unwrap_or(rest);
    path.ends_with(".rs") && (in_tests_dir || name == "tests.rs" || name.ends_with("_tests.rs"))
}

/// Every private host or address `text` names, as `path:line: name is why`:
/// once per URL that names it and once per other mention, a bare mention of
/// a host a URL on the same line names counting with the URL.
fn findings(path: &str, text: &str) -> Vec<String> {
    if rust_test_file(path) {
        return Vec::new();
    }
    let mut found = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let hosts = url_hosts(line);
        let bare = line
            .split(|c: char| !host_char(c))
            .filter(|token| token.contains('.') && !hosts.contains(token))
            .map(|token| (token, false));
        for (name, in_url) in hosts.iter().map(|host| (*host, true)).chain(bare) {
            if let Some(why) = private(name, in_url) {
                found.push(format!("{path}:{}: {name} is {why}", index + 1));
            }
        }
    }
    found
}

#[test]
fn no_tracked_file_names_a_private_host_or_address() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let named: Vec<String> = tracked_text::texts(&root)
        .flat_map(|(path, text)| findings(&path, &text))
        .collect();
    assert_eq!(
        named,
        Vec::<String>::new(),
        "#532: the repository is public, and a shipped file names no private \
         host or address; a route is named by its route name"
    );
}

#[test]
fn every_spelling_is_found_and_a_placeholder_is_not() {
    let planted = [
        "the route reads `http://spark:30000/v1`.",
        "baseURL: https://key@gpu-box:8888/v1",
        "reach it at 192.168.1.20 by hand, or http://10.0.0.7:8000/v1",
        "172.20.1.1 and 172.32.0.1",
        "100.100.1.2, 100.128.0.1 and 169.254.1.1",
        "spark.lan or http://dgx.local:30000 or spark.tail1234.ts.net.",
        "SPARK.Internal, http://box.home.arpa/v1 and dgx.localdomain",
        "record.expected.local, `.lan` and `.internal`",
        "https://host/x http://localhost:8080 https://hóst/x https://<host>/v1",
        "file:///tmp/x and https://token-plan.ap-southeast-1.maas.aliyuncs.com",
        "adapters.rs:5863, 12:30 and 8.8.8.8",
        "the route—http://dgx/v1",
    ]
    .join("\n");
    let expected = |path: &str| {
        [
            format!("{path}:1: spark is a single-label host"),
            format!("{path}:2: gpu-box is a single-label host"),
            format!("{path}:3: 10.0.0.7 is a private address"),
            format!("{path}:3: 192.168.1.20 is a private address"),
            format!("{path}:4: 172.20.1.1 is a private address"),
            format!("{path}:5: 100.100.1.2 is a shared address"),
            format!("{path}:5: 169.254.1.1 is a link-local address"),
            format!("{path}:6: dgx.local is under a private-use suffix"),
            format!("{path}:6: spark.lan is under a private-use suffix"),
            format!("{path}:6: spark.tail1234.ts.net. is under a private-use suffix"),
            format!("{path}:7: box.home.arpa is under a private-use suffix"),
            format!("{path}:7: SPARK.Internal is under a private-use suffix"),
            format!("{path}:7: dgx.localdomain is under a private-use suffix"),
            format!("{path}:12: dgx is a single-label host"),
        ]
    };
    for path in [
        "docs/guides/provider-adapters.md",
        "docs/decisions/0036-egress-is-a-property-of-the-route.md",
        "crates/brokkr-cli/tests/fixtures/endpoint.json",
        "crates/example/src/lib.rs",
        "crates/example/src/tests_support.rs",
    ] {
        assert_eq!(findings(path, &planted), expected(path), "{path}");
    }
    for path in [
        "crates/brokkr-cli/tests/private_hosts.rs",
        "crates/brokkr-protocol/src/adapters/tests.rs",
        "crates/brokkr-runtime/src/engine/resume_tests.rs",
    ] {
        assert_eq!(findings(path, &planted), Vec::<String>::new(), "{path}");
    }
}
