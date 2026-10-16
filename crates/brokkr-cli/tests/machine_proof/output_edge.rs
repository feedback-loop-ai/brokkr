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
    // standard error is masked raw and again as the text it sanitizes to.
    let texts = ["named", "prepared", "redact"].map(|function| at(&output, function));
    let masked = [texts.concat(), at(&stderr, "shown"), at(&stderr, "shown")];
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
fn the_servers_own_standard_error_reaches_the_drain_alone_and_the_ledger_is_bound_once() {
    // The launch gives the server a standard error of its own, which `serve`
    // hands one drain from the server's startup on, and nothing else reads;
    // its tail is kept in one place, for the lifetime to write (U6f1b).
    let entry = PathBuf::from("brokkr-protocol/src/hands/namespace/entry.rs");
    let mut launched = production_calls(".stderr(");
    launched.retain(|(file, _)| *file == entry);
    assert_eq!(launched, [(entry, "launch<'e>".to_string())]);
    let at = |file: &str, function: &str| vec![(PathBuf::from(file), function.to_string())];
    let readiness = "brokkr-cli/src/broker/session/readiness.rs";
    let ready = "ready<T, B, E: From<Refusal>>";
    assert_eq!(broker_calls("launched.errors"), at(readiness, ready));
    let rpc = "brokkr-cli/src/broker/rpc.rs";
    assert_eq!(broker_calls("Drain::new("), at(rpc, "serving"));
    assert_eq!(broker_calls("stderr.tail("), at(rpc, "serving"));
    // The session's ledger is bound by its binder, once, and the call path
    // it hands that ledger to binds it no more (operator ruling 2026-10-13).
    let lifetime = "brokkr-cli/src/broker/session/lifetime.rs";
    assert_eq!(broker_calls("ledger.bind("), at(lifetime, "bind"));
}

#[test]
fn the_brokers_own_standard_error_has_one_writer_and_one_voice() {
    // Every byte `serve` writes to its standard error is a line its voice
    // passed, sanitized, masked and guarded, and is written by one function,
    // which `serve` hands its lifetime as its one writing, called while it
    // holds the bindings: before the ledger closes for a served session's
    // tail, after it for a failed one's (U6f1b; MB4; operator ruling
    // 2026-10-14).
    // No broker file prints, and its standard input and output are the
    // call path's and the observer's socket's alone, beside the engine's
    // provisioning survey's one request and one reply (U7c0b).
    let stderr = "brokkr-cli/src/broker/session/stderr.rs";
    let lifetime = "brokkr-cli/src/broker/session/lifetime.rs";
    let at = |file: &str, functions: &[&str]| -> Vec<(PathBuf, String)> {
        let pinned = functions
            .iter()
            .map(|function| (PathBuf::from(file), function.to_string()));
        pinned.collect()
    };
    assert_eq!(broker_calls("std::io::stderr("), at(stderr, &["told"]));
    assert_eq!(broker_calls("stderr::told("), at(lifetime, &["serve"]));
    // The writing is set up once, its descriptor or why it could not be,
    // which is a delivery failure (operator ruling 2026-10-14).
    assert_eq!(broker_calls("told_through("), at(stderr, &["told"]));
    assert_eq!(broker_calls("told_to("), at(stderr, &["told_through"]));
    assert_eq!(broker_calls("Voice::new("), at(stderr, &["last", "new"]));
    // Every frame the harness is told, and every line of the broker's own
    // standard error, is cut at its safe points by the one rule
    // (operator ruling 2026-10-13).
    let output = at("brokkr-cli/src/broker/output.rs", &["framed"]);
    assert_eq!(broker_calls("Pieces::new("), output);
    assert_eq!(broker_calls("pieces::cut("), at(stderr, &["cut"]));
    assert_eq!(broker_calls(".said("), at(stderr, &["keep", "last"]));
    assert_eq!(broker_calls("Safe::new("), at(stderr, &["shown"]));
    let session = "brokkr-cli/src/broker/session.rs";
    let observer = at(session, &["handed", "tethered", "tethered"]);
    // The harness's are held apart for the session alone, a stream that
    // cannot be set up its delivery failure (operator ruling 2026-10-14).
    let rpc = at("brokkr-cli/src/broker/rpc.rs", &["held_apart"]);
    let serving = at("brokkr-cli/src/broker/rpc.rs", &["serving"]);
    assert_eq!(broker_calls(", held_apart)"), serving);
    let survey = at("brokkr-cli/src/broker/survey.rs", &["run"]);
    assert_eq!(
        broker_calls("std::io::stdout("),
        [rpc.clone(), observer, survey].concat()
    );
    // The survey's watcher reads the rest of its input to its end (U7c).
    let tether = at("brokkr-cli/src/broker/survey.rs", &["run", "tethered"]);
    assert_eq!(broker_calls("std::io::stdin("), [rpc, tether].concat());
    // Those streams are waited on by poll, never by their status flags,
    // which every holder of each shares: only the broker's own pipes to its
    // box, which no other process reads or writes its end of, are set so.
    let readiness = PathBuf::from("brokkr-cli/src/broker/session/readiness.rs");
    let flagged = broker_calls("ioctl_fionbio(")
        .into_iter()
        .map(|(file, _)| file);
    assert_eq!(flagged.collect::<Vec<_>>(), vec![readiness; 3]);
    let printing = ["print!(", "println!(", "eprint!(", "eprintln!(", "dbg!("];
    let printed: Vec<(PathBuf, &str)> = broker_files()
        .into_iter()
        .flat_map(|(file, text)| {
            let found = printing.iter().filter(|needle| text.contains(*needle));
            found
                .map(|needle| (file.clone(), *needle))
                .collect::<Vec<_>>()
        })
        .collect();
    assert_eq!(printed, []);
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
