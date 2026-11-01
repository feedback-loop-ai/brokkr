//! A managed name's `-` (U9b1d; controller ruling R4 of 2026-11-01).

use super::managed_patterns;

const NAME: &str = "names a tool that is not a plain name of ASCII letters, digits and '_' \
                    leading with a letter, within 128 bytes";

/// A broker's served name and its tools read as managed names, with `-`
/// inside the server part of an `mcp__` name; a `-` anywhere else, or
/// leading that server part, refuses as any other unplain name does.
#[test]
fn a_dash_reads_only_inside_an_mcp_server_name_and_never_first() {
    let allowed = "mcp__brokkr__workspace,mcp__cap-library-docs,mcp__cap-web__resolve(x)";
    assert_eq!(
        managed_patterns(allowed),
        Ok(vec![
            "mcp__brokkr__workspace",
            "mcp__cap-library-docs",
            "mcp__cap-web__resolve(x)"
        ])
    );
    for value in [
        "Web-Search",
        "Bash-(x)",
        "mcp-x",
        "mcp__-cap",
        "mcp__-cap__resolve",
        "mcp__cap-docs__re-solve",
        "mcp__cap-docs__-",
        "mcp_-cap",
        "-mcp__cap",
    ] {
        assert_eq!(managed_patterns(value), Err(NAME), "{value:?}");
    }
}
