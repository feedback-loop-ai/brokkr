//! Doctor's own words where nothing the compile does stands in front of
//! them. U8m: a declared server's, so a URL's userinfo, which no dialect
//! that loads carries, is withheld all the same. U9b1a: a serving's line
//! under U1g1's strict verdict, which past the lifted fence (Linux) no
//! surveyed bundle reaches with a refusal, as the compile refuses the
//! site first.

use std::collections::BTreeMap;

use brokkr_protocol::adapters::StrictCause;
use brokkr_protocol::native_controls::Provenance;
use brokkr_runtime::capabilities::{Connection, McpServer, NativePlan, Outcome, SiteAsks};
use brokkr_runtime::SeatClass;

use super::{report_serving, server_words, STRICT};
use crate::doctor::Report;

/// A marker planted where a credential would sit.
const PLANTED: &str = "U8M-MARKER";

/// Doctor's words for a server reached by `connection`.
fn named(connection: Connection) -> String {
    let (version, secrets) = ("1.0.0".to_string(), Vec::new());
    let server = McpServer {
        connection,
        version,
        secrets,
        retained: false,
    };
    server_words(&server)
}

/// A URL is named by its scheme, host, port and path only: a userinfo
/// before the authority's last `@`, a query and a fragment are withheld,
/// each alone or together, with the one marker; an `@` in the path is the
/// path's, and a URL that carries none of them is named whole. A stdio
/// server is its program, with how many arguments follow it where any do.
#[test]
fn a_server_is_named_without_its_userinfo_query_fragment_or_arguments() {
    let withheld = "(user, query and fragment withheld)";
    let urls = [
        (
            format!("https://{PLANTED}:{PLANTED}@a@docs.example:8443/mcp?{PLANTED}#{PLANTED}"),
            format!("at URL 'https://docs.example:8443/mcp' {withheld}"),
        ),
        (
            format!("http://{PLANTED}@docs.example"),
            format!("at URL 'http://docs.example' {withheld}"),
        ),
        (
            format!("https://docs.example/mcp#{PLANTED}"),
            format!("at URL 'https://docs.example/mcp' {withheld}"),
        ),
        (
            "https://docs.example/a@b".to_string(),
            "at URL 'https://docs.example/a@b'".to_string(),
        ),
    ];
    for (url, words) in urls {
        assert_eq!(named(Connection::Url(url)), words);
    }
    let argv =
        |argv: &[&str]| Connection::Stdio(argv.iter().map(|part| part.to_string()).collect());
    assert_eq!(named(argv(&["/opt/docs"])), "'/opt/docs' (stdio)");
    assert_eq!(
        named(argv(&["/opt/docs", "--key", PLANTED])),
        "'/opt/docs' (stdio, 2 further argument(s) withheld)"
    );
}

/// A serving U1g1's strict check admits is reported ok, and one it refuses
/// is warned with its cause, on every host: the warning is what a seat's
/// line reads on macOS, where the fence stands and the compile admits the
/// site.
#[test]
fn a_serving_the_strict_check_refuses_is_warned_with_its_cause() {
    let asks = SiteAsks::at(SeatClass::Work, "work", None, None).expect("an inline site's asks");
    let outcome = Outcome {
        provider: "claude".to_string(),
        harness: "claude".to_string(),
        model: None,
        held: BTreeMap::new(),
        not_held: BTreeMap::new(),
        notices: Vec::new(),
        native: NativePlan::Unmeasured {
            declaration: None,
            reason: "unmeasured here".to_string(),
            provenance: Provenance::default(),
        },
    };
    let cause = StrictCause::Unmeasured {
        provider: "claude".to_string(),
        shape: "replacement".to_string(),
    };
    let mut report = Report {
        healthy: true,
        lines: Vec::new(),
    };
    report_serving(&mut report, "work", &asks, &outcome, None);
    report_serving(&mut report, "work", &asks, &outcome, Some(&cause));
    let seat = "capabilities seat 'work' claude model none: office 'work' asks: nothing · \
                holds: nothing · not held: nothing ·";
    assert_eq!(
        report.lines,
        [
            format!("ok       {seat} {STRICT} admits every shape it can be served as"),
            format!(
                "warn     {seat} {STRICT} refuses it: provider 'claude' has no measured strict \
                 MCP configuration for 'replacement'"
            ),
        ]
    );
    assert!(report.healthy, "a warning leaves doctor healthy");
}
