//! The filtered protocol's startup through `serve` (decision 0065 slice two
//! U6e; MB3, CR3, SD3), on a real box: once the server has started in its
//! box it is initialized and its catalog listed within the startup's
//! absolute deadline, and then the box is ended and serving still refuses
//! before any message reaches the harness. A server off its pinned
//! version, one lacking a granted tool, one that speaks no MCP, one that
//! runs but lacks a dependency inside its box, and one that never answers
//! each end the session with their own cause, latched in the ledger, after
//! the one lookup: distinct from the pre-secret refusals, which look
//! nothing up, and from an interpreter absent from the box, which starts
//! nothing.

use std::time::{Duration, Instant};

use brokkr_protocol::broker::ledger::{Disposition, Failure};

use super::ledger::{failed_with, sealed_ledger, Proved};
use super::observer::within;
use super::readiness::{boxed, Reads};
use super::{admitted, ended, refused, skip, unservable, Refusal, Sealed, SPEAKS};

/// A sealed fixture whose server, started, runs `said` in the box's own
/// shell, and its plan's digest; none, declared skipped, where no box
/// stands.
fn speaking(said: &str) -> Option<(Sealed, String)> {
    if let Some(reason) = unservable() {
        skip(&reason);
        return None;
    }
    let sealed = Sealed::new();
    let server = sealed.path("opt/docs/bin/docs-mcp");
    std::fs::write(server, format!("#!/bin/sh\n{said}")).unwrap();
    let digest = sealed.seal_bytes(sealed.plan().to_string().as_bytes());
    Some((sealed, digest))
}

/// `serve`'s answer for the server that runs `said`, the lookups its store
/// saw, and what its ledger proves once nothing of its box is left; none,
/// declared skipped, where no box stands.
fn served(said: &str) -> Option<((Option<i32>, String), usize, Proved)> {
    let (sealed, digest) = speaking(said)?;
    let reads = Reads::of(&sealed);
    let answer = sealed.serve(&digest);
    assert!(within(|| boxed(&sealed).is_empty()));
    Some((answer, reads.lookups(), sealed_ledger(&sealed)))
}

/// A ledger closed Failed with `failure` and no call.
fn failed(failure: Failure) -> Proved {
    (vec![], Some(Disposition::Failed(failure)), None)
}

#[test]
fn a_server_that_initializes_is_listed_and_serving_still_refuses_before_the_harness() {
    // The control: the fixture's server answers initialize at the pinned
    // version and lists both granted tools beside one it is not granted.
    // `serve` writes nothing to its harness ([`Sealed::serve_again`]).
    let Some(healthy) = served(SPEAKS) else {
        return;
    };
    let incomplete = failed_with(Refusal::ServingIncomplete);
    assert_eq!(healthy, (admitted(), 1, incomplete));
}

#[test]
fn a_server_off_its_pin_or_lacking_a_granted_tool_ends_the_session_with_its_own_cause() {
    let unpinned = SPEAKS.replace(r#""version":"1.4.2""#, r#""version":"1.4.3""#);
    let unsupported = SPEAKS.replace("2025-06-18", "2024-11-05");
    let lacking = SPEAKS.replace(
        r#"{"name":"read","inputSchema":{"type":"object"}}"#,
        r#"{"name":"other"}"#,
    );
    let missing = "MCP server does not offer granted tool 'read'";
    let unsupported_text = "MCP protocol version is not supported by this broker";
    let cases = [
        (
            "unpinned",
            unpinned,
            ended(Failure::Version),
            Failure::Version,
        ),
        (
            "unsupported",
            unsupported,
            (Some(1), format!("error: {unsupported_text}\n")),
            Failure::Protocol,
        ),
        (
            "lacking",
            lacking,
            (Some(1), format!("error: {missing}\n")),
            Failure::Protocol,
        ),
    ];
    for (case, said, answer, cause) in cases {
        let Some(ran) = served(&said) else {
            return;
        };
        assert_eq!((case, ran), (case, (answer, 1, failed(cause))));
    }
}

#[test]
fn a_running_server_lacking_a_dependency_or_speaking_no_mcp_is_the_protocols_cause() {
    // A dependency absent from the box, found only once the server runs:
    // its shell cannot run it, and the server ends without a word of MCP.
    // Its interpreter the box holds, so its exec succeeded (MB3); an
    // interpreter absent from the box is the boxed-exec cause instead
    // (`handoff`).
    let dependency = "/opt/docs/lib/helper --serve\n";
    let corrupt = "read -r line\necho 'this is no frame'\nread -r line\n";
    for (case, said) in [("dependency", dependency), ("corrupt", corrupt)] {
        let Some(ran) = served(said) else {
            return;
        };
        let protocol = (ended(Failure::Protocol), 1, failed(Failure::Protocol));
        assert_eq!((case, ran), (case, protocol));
    }
}

#[test]
fn a_server_that_never_answers_ends_at_the_startup_deadline() {
    // It reads every request and answers none, so its box stands until
    // the startup's absolute deadline, which admission and readiness have
    // already spent from: thirty seconds from `serve`'s start, and no more
    // than a slice after.
    let Some((sealed, digest)) = speaking("while read -r line; do :; done\n") else {
        return;
    };
    let reads = Reads::of(&sealed);
    let begun = Instant::now();
    let answer = sealed.serve(&digest);
    let taken = begun.elapsed();
    assert_eq!(answer, ended(Failure::Timeout));
    let bounded = (taken >= Duration::from_secs(29)) & (taken < Duration::from_secs(32));
    assert!(bounded, "{taken:?}");
    assert!(within(|| boxed(&sealed).is_empty()));
    assert_eq!(
        (reads.lookups(), sealed_ledger(&sealed)),
        (1, failed(Failure::Timeout))
    );
}

#[test]
fn a_refusal_before_the_box_looks_nothing_up_and_starts_no_server() {
    // The control beside the endings above: a refusal before any secret,
    // here a fixed key's, starts no server to initialize.
    let Some((sealed, _)) = speaking(SPEAKS) else {
        return;
    };
    let reads = Reads::of(&sealed);
    let answer = sealed.with("/secrets", serde_json::json!(["HOME"]));
    assert_eq!(answer, refused(Refusal::StartupInputs));
    let closed = failed_with(Refusal::StartupInputs);
    assert_eq!((reads.lookups(), sealed_ledger(&sealed)), (0, closed));
}
