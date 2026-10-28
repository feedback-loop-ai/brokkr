//! U8m: doctor's words for a declared server, read where the dialect
//! contract's own refusal cannot stand in front of them: a URL's userinfo,
//! which no dialect that loads carries, is withheld all the same.

use brokkr_runtime::capabilities::{Connection, McpServer};

use super::server_words;

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
