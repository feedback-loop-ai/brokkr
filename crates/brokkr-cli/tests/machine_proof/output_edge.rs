//! What the broker's output edge masks with, found by grep as decision
//! 0012 prescribes (decision 0065 slice two, U6f1a; MB4, CR2): the shared
//! masker over every response and over the server's standard error, the
//! one canonical serializer for the prepared buffer, and no encoding of a
//! secret of the broker's own.

use std::path::{Path, PathBuf};

use super::egress::production_calls;

/// The production files of the broker's module tree, by their path below
/// the crates directory.
fn broker_files() -> Vec<(PathBuf, String)> {
    let crates_dir = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let (mut files, mut stack) = (Vec::new(), vec![crates_dir.join("brokkr-cli/src/broker")]);
    stack.push(crates_dir.join("brokkr-cli/src/broker.rs"));
    while let Some(path) = stack.pop() {
        if path.is_dir() {
            stack.extend(
                std::fs::read_dir(&path)
                    .unwrap()
                    .map(|entry| entry.unwrap().path()),
            );
            continue;
        }
        if !tests(&path) {
            let below = path.strip_prefix(crates_dir).unwrap().to_path_buf();
            files.push((below, std::fs::read_to_string(&path).unwrap()));
        }
    }
    files
}

/// Whether `path` is test code: a `tests.rs`, or a file below a `tests`
/// directory.
fn tests(path: &Path) -> bool {
    let named = path.file_name().is_some_and(|name| name == "tests.rs");
    named | path.components().any(|part| part.as_os_str() == "tests")
}

/// Each production broker file calling `needle`, by the function it sits
/// in.
fn broker_calls(needle: &str) -> Vec<(PathBuf, String)> {
    let mut calls = production_calls(needle);
    calls.retain(|(file, _)| file.starts_with("brokkr-cli/src/broker") & !tests(file));
    calls.sort();
    calls
}

#[test]
fn the_output_edge_masks_with_the_shared_masker_and_serializes_once() {
    let output = PathBuf::from("brokkr-cli/src/broker/output.rs");
    let stderr = PathBuf::from("brokkr-cli/src/broker/session/stderr.rs");
    let at = |file: &PathBuf, function: &str| vec![(file.clone(), function.to_string())];
    assert_eq!(broker_calls("mask_json("), at(&output, "freely"));
    assert_eq!(
        broker_calls("canonical::to_bytes("),
        at(&output, "prepared")
    );
    assert_eq!(broker_calls("parse_strict("), at(&output, "checked"));
    // Free text and standard error are masked by the one conversion
    // protocol/secret.rs's mask_text makes, raw bytes first; a line of
    // standard error is masked raw and again as the text it decodes to.
    let texts = ["named", "prepared", "redact"].map(|function| at(&output, function));
    let masked = [texts.concat(), at(&stderr, "keep"), at(&stderr, "keep")];
    assert_eq!(broker_calls("mask_text("), masked.concat());
    assert_eq!(broker_calls("mask_bytes("), []);
    // One final guard, protocol/secret.rs's: the edge holds one for its
    // frames, ids and names and the drain one for its lines, the ledger
    // guards every record by it after the shared masker, and a number is
    // read as a bound value there alone.
    let calls = PathBuf::from("brokkr-cli/src/broker/rpc/calls.rs");
    let guards = [at(&calls, "served_within<'b>"), at(&stderr, "new")];
    assert_eq!(broker_calls("Guard::new("), guards.concat());
    let ledger = PathBuf::from("brokkr-protocol/src/broker/ledger.rs");
    let guard = PathBuf::from("brokkr-protocol/src/secret/guard.rs");
    let protocol = |needle: &str| {
        let mut calls = production_calls(needle);
        calls.retain(|(file, _)| file.starts_with("brokkr-protocol/src"));
        calls.sort();
        calls
    };
    let ledgers = [at(&ledger, "bind"), at(&ledger, "open_with")];
    assert_eq!(protocol("Guard::new("), ledgers.concat());
    assert_eq!(protocol("mask_json(&mut masked"), at(&ledger, "masked"));
    assert_eq!(protocol(".valued("), at(&guard, "passes<B>"));
}

#[test]
fn no_broker_file_encodes_a_secret_of_its_own() {
    // The shared encodings are protocol/secret.rs's alone: a broker file
    // that wrote one, or read a secret's value, would mask by a copy.
    let forbidden = [
        "fn enc_",
        "NEEDLE_ENCODINGS",
        "expose_for_spawn",
        "mask_needles",
    ];
    let found: Vec<(PathBuf, &str)> = broker_files()
        .into_iter()
        .flat_map(|(file, text)| {
            let named = forbidden.iter().filter(|needle| text.contains(*needle));
            named
                .map(|needle| (file.clone(), *needle))
                .collect::<Vec<_>>()
        })
        .collect();
    assert_eq!(found, []);
}
