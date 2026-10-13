//! Where a secret leaves its typed holder in production code, found by
//! grep as decision 0012 prescribes: the one plaintext accessor's call,
//! and the one binding frame that carries what it binds to a server box
//! (decision 0065 slice two, U6c8; MB4).

use std::path::{Path, PathBuf};

/// Each call of `needle` in each crate's `src/` tree, tests aside, by its
/// file below the crates directory and the function it sits in. A
/// definition is not a call.
pub(super) fn production_calls(needle: &str) -> Vec<(PathBuf, String)> {
    let crates_dir = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let (mut call_sites, mut stack) = (Vec::new(), vec![crates_dir.to_path_buf()]);
    while let Some(dir) = stack.pop() {
        let entries = std::fs::read_dir(&dir).unwrap();
        for path in entries.map(|entry| entry.unwrap().path()) {
            let name = path.file_name().unwrap().to_str().unwrap_or_default();
            let tests = (name == "tests.rs") | name.ends_with("_tests.rs");
            let src = path.components().any(|c| c.as_os_str() == "src");
            if path.is_dir() {
                stack.push(path);
            } else if name.ends_with(".rs") & src & !tests {
                let content = std::fs::read_to_string(&path).unwrap();
                // A call sits in the nearest function opened before it.
                let calls = content.match_indices(needle).map(|(at, _)| &content[..at]);
                for before in calls.filter(|before| !before.ends_with("fn ")) {
                    let enclosing = before.rfind("fn ").map_or("", |fn_at| &before[fn_at + 3..]);
                    let function = enclosing.split('(').next().unwrap_or_default();
                    let file = path.strip_prefix(crates_dir).unwrap().to_path_buf();
                    call_sites.push((file, function.to_string()));
                }
            }
        }
    }
    call_sites
}

#[test]
fn the_binding_frame_has_one_sender_and_one_receiver() {
    // What the injector binds reaches a server box only as the one frame
    // the broker sends after readiness and the bootstrap receives before
    // its exec: each side called once, from its own consumer.
    let sender = PathBuf::from("brokkr-cli/src/broker/session/readiness.rs");
    let sent = production_calls(&["send_", "environment("].concat());
    assert_eq!(sent, [(sender, "delivered<E: From<Refusal>>".to_string())]);
    let receiver = PathBuf::from("brokkr-cli/src/broker/bootstrap.rs");
    let received = production_calls(&["exec_", "confined("].concat());
    assert_eq!(received, [(receiver, "served".to_string())]);
}
