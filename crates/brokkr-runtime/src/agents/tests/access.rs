//! The office `access` carrier (#896 round 2).

use serde_json::json;

use super::{agent_body, ready};
use crate::agents::{Access, Library, LibraryError};

/// #896 round 2: a boxed office's access is typed at the loader edge,
/// absent means writes, and any other word is refused rather than read as
/// absent. Round 3 keeps the refusal typed and pins its complete Display
/// once (decision 0071 rulings 8 and 9).
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
    let error = Library::load(&tree.library_root()).unwrap_err();
    let message = format!(
        "agent 'tester' ({}) 'access' is \"workspace-write\"; the vocabulary is read-only or \
         writes (decision 0071 rulings 2 and 3)",
        tree.library_root().join("tester.json").display()
    );
    match &error {
        LibraryError::Invalid(said) => assert_eq!(said, &message),
        other => panic!("the access refusal is typed, got {other:?}"),
    }
    assert_eq!(error.to_string(), message);
}
