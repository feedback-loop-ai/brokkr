//! What the engine binds the box's hands to in the fixtures, and the
//! documents and arguments those transports expand to, written out by
//! hand. Moved out of the parent suite, which sits at its line baseline,
//! when the transport gained the reach sealed at admission (decision 0065
//! slice two, U8a2c).

use super::Transport;

/// The typed hands declaration the fixtures' sites carry.
pub(super) static SPEC: crate::hands::HandsSpec = crate::hands::HandsSpec {
    network: false,
    binds: Vec::new(),
};

/// The reach the fixtures' boxes are sealed: one that seals nothing, as
/// the driver binds where its input carries none (U8a2c).
pub(super) static REACH: std::sync::LazyLock<crate::hands::Sealed> =
    std::sync::LazyLock::new(Default::default);

/// What the engine binds the box's hands to in the fixtures.
pub(super) fn transport() -> Transport<'static> {
    Transport {
        brokkr: std::path::Path::new("/bin/brokkr"),
        workdir: std::path::Path::new("/w"),
        artifacts: std::path::Path::new("/a"),
        sealed: &REACH,
        spec: &SPEC,
    }
}

/// The MCP document [`transport`] binds, written out by hand.
pub(super) const MCP_DOCUMENT: &str = "{\"mcpServers\":{\"brokkr\":{\"args\":[\"hands\",\
    \"serve\",\"--workdir\",\"/w\",\"--artifacts\",\"/a\",\"--reach\",\
    \"{\\\"home\\\":\\\"\\\",\\\"worktree\\\":{\\\"path\\\":\\\"\\\",\\\"identity\\\":null},\
    \\\"git\\\":null,\\\"binds\\\":[]}\",\"--spec\",\"{\\\"binds\\\":[],\\\"kind\\\":\
    \\\"workspace\\\",\\\"network\\\":false}\"],\"command\":\"/bin/brokkr\"}}}";

/// The server's arguments [`transport`] binds, as the TOML array Codex is
/// handed, written out by hand.
pub(super) const SERVER_ARGUMENTS: &str = "[\"hands\",\"serve\",\"--workdir\",\"/w\",\
    \"--artifacts\",\"/a\",\"--reach\",\
    \"{\\\"home\\\":\\\"\\\",\\\"worktree\\\":{\\\"path\\\":\\\"\\\",\\\"identity\\\":null},\
    \\\"git\\\":null,\\\"binds\\\":[]}\",\"--spec\",\
    \"{\\\"binds\\\":[],\\\"kind\\\":\\\"workspace\\\",\\\"network\\\":false}\"]";

/// A transport whose executable and workdir hold what a TOML basic string
/// must escape: a literal backslash-u002f, a quote and a backslash (R2).
pub(super) fn escaped_transport() -> Transport<'static> {
    Transport {
        brokkr: std::path::Path::new("/opt/a\\u002fb/brokkr"),
        workdir: std::path::Path::new("/w/a\"b\\c"),
        artifacts: std::path::Path::new("/a"),
        sealed: &REACH,
        spec: &SPEC,
    }
}

/// The Codex hands [`escaped_transport`] binds, as TOML text, written out
/// by hand from TOML's basic-string escapes.
pub(super) const ESCAPED_COMMAND: &str = "mcp_servers.brokkr.command=\"/opt/a\\\\u002fb/brokkr\"";
pub(super) const ESCAPED_ARGUMENTS: &str = "[\"hands\",\"serve\",\"--workdir\",\
    \"/w/a\\\"b\\\\c\",\"--artifacts\",\"/a\",\"--reach\",\
    \"{\\\"home\\\":\\\"\\\",\\\"worktree\\\":{\\\"path\\\":\\\"\\\",\\\"identity\\\":null},\
    \\\"git\\\":null,\\\"binds\\\":[]}\",\"--spec\",\
    \"{\\\"binds\\\":[],\\\"kind\\\":\\\"workspace\\\",\\\"network\\\":false}\"]";
