//! The office `access` carrier (#896 round 2).

use serde_json::json;

use super::{agent_body, ready};
use crate::agents::Access;

/// #896 round 2: a boxed office's access is typed at the loader edge,
/// absent means writes, and any other word is refused rather than read as
/// absent.
#[test]
fn an_office_access_is_decoded_once_and_absent_means_writes() {
    let tree = ready();
    assert_eq!(
        tree.library().agent("tester").unwrap().access,
        Access::Writes
    );
    let mut body = agent_body();
    body["access"] = json!("read-only");
    tree.write("agents/tester.json", &body);
    assert_eq!(
        tree.library().agent("tester").unwrap().access,
        Access::ReadOnly
    );
    body["access"] = json!("workspace-write");
    tree.write("agents/tester.json", &body);
    let said = tree.library_error();
    assert!(said.contains("'access' is \"workspace-write\""), "{said}");
    assert!(said.contains("read-only or writes"), "{said}");
}
