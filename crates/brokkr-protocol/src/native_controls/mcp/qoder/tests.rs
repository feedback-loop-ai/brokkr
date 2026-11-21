use std::path::Path;

use super::*;
use serde_json::json;

/// The fragment is the strict flag, the engine's file and the empty
/// setting sources, in that order and all three together (Q07, Q14).
#[test]
fn the_isolation_is_the_strict_flag_the_file_and_the_empty_sources() {
    assert_eq!(
        isolation("/tmp/x/mcp.json"),
        [
            "--strict-mcp-config",
            "--mcp-config",
            "/tmp/x/mcp.json",
            "--setting-sources",
            "",
        ]
    );
}

/// No hands is the empty set: the measured empty document and no grant.
#[test]
fn no_hands_serve_the_empty_document_and_grant_nothing() {
    assert_eq!(
        served(None),
        Some(Served {
            document: json!({"mcpServers": {}}),
            grants: Vec::new(),
        })
    );
}

/// What [`served`] gives the default hands in `/w`, artifacts in `/a`,
/// run by `brokkr`.
fn served_by(brokkr: &Path) -> Option<Served> {
    let (spec, reach) = (Default::default(), Default::default());
    let (workdir, artifacts) = (Path::new("/w"), Path::new("/a"));
    let sealed = &reach;
    let spec = &spec;
    served(Some(Transport {
        brokkr,
        workdir,
        artifacts,
        sealed,
        spec,
    }))
}

/// The box's hands are the one server, written by the set's builder, and
/// their one tool is granted as Qoder names an MCP tool (Q12 to Q15).
#[test]
fn the_hands_serve_their_document_and_grant_their_one_tool() {
    let (spec, reach) = (Default::default(), Default::default());
    let args = crate::hands::serve_args(Path::new("/w"), Path::new("/a"), &reach, &spec);
    assert_eq!(
        served_by(Path::new("/bin/brokkr")),
        Some(Served {
            document: json!({"mcpServers": {"brokkr": {"command": "/bin/brokkr", "args": args}}}),
            grants: vec!["mcp__brokkr__workspace".to_string()],
        })
    );
}

/// Hands whose executable is not UTF-8 are served nothing: a lossy
/// document would bind another program.
#[cfg(unix)]
#[test]
fn hands_no_document_value_represents_are_served_nothing() {
    use std::os::unix::ffi::OsStrExt;
    let lossy = Path::new(std::ffi::OsStr::from_bytes(b"/bin/\xffbrokkr"));
    assert_eq!(served_by(lossy), None);
}
