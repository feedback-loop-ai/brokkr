//! The provisioning observer's request and reply (U7c0b): one closed,
//! bounded request read whole, each way it fails to read answered with
//! its own closed cause, and each refusal before a box is observed
//! answered with its own. Each malformed request differs from a request
//! that reads by its one defect alone.

use brokkr_protocol::broker::{Network, Reach};
use serde_json::json;

use super::*;

/// A request naming `launch` under `home`, with one sealed writer.
fn asking(launch: &str, home: &str) -> Survey {
    Survey {
        launch: launch.into(),
        home: home.into(),
        reach: Reach {
            writable: Vec::new(),
            readable: Vec::new(),
        },
        network: Network::Isolated,
        writers: vec![1000],
    }
}

/// [`asking`]'s wire form, `sh` under `/home/op`, and that it reads.
fn written() -> String {
    let text = serde_json::to_string(&asking("sh", "/home/op")).unwrap();
    assert_eq!(read(text.as_bytes()), Ok(asking("sh", "/home/op")));
    text
}

/// What `request` makes of `text`.
fn read(text: &[u8]) -> Result<Survey, Unread> {
    request(&mut &text[..])
}

#[test]
fn a_request_is_read_whole_within_its_bound() {
    // Padded to the bound exactly it is read; one byte more is not.
    let mut padded = format!("{}\n", written()).into_bytes();
    padded.resize(REQUEST_BYTES_MAX as usize, b' ');
    assert_eq!(read(&padded), Ok(asking("sh", "/home/op")));
    padded.push(b' ');
    assert_eq!(read(&padded), Err(Unread::Oversized));
}

#[test]
fn a_request_that_ends_inside_its_record_is_truncated() {
    let text = written();
    assert_eq!(
        read(&text.as_bytes()[..text.len() - 1]),
        Err(Unread::Truncated)
    );
    assert_eq!(read(b""), Err(Unread::Truncated));
}

#[test]
fn a_request_repeating_a_field_is_malformed() {
    let twice = written().replacen('{', r#"{"launch":"bash","#, 1);
    assert_eq!(read(twice.as_bytes()), Err(Unread::Malformed));
}

#[test]
fn a_request_naming_a_field_the_record_does_not_is_malformed() {
    written();
    let mut widened = serde_json::to_value(asking("sh", "/home/op")).unwrap();
    widened["plan"] = json!("/p");
    assert_eq!(read(widened.to_string().as_bytes()), Err(Unread::Malformed));
}

#[test]
fn a_request_followed_by_another_is_malformed() {
    let text = written();
    let doubled = format!("{text}\n{text}\n");
    assert_eq!(read(doubled.as_bytes()), Err(Unread::Malformed));
}

/// Input that cannot be read.
struct Broken;

impl Read for Broken {
    fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("the pipe broke"))
    }
}

#[test]
fn input_that_cannot_be_read_is_unread_input() {
    assert_eq!(request(&mut Broken), Err(Unread::Input));
}

/// A running binary that cannot be named.
fn nameless() -> std::io::Result<PathBuf> {
    Err(std::io::Error::from(std::io::ErrorKind::NotFound))
}

#[test]
fn a_missing_program_a_relative_home_or_an_unread_bootstrap_refuses_with_its_cause() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path().to_str().unwrap().to_string();
    let cases = [
        (
            "no program",
            asking("no-such-mcp-server", &home),
            Refusal::ProgramTree,
        ),
        // `src` resolves from the test's cwd, the crate: only its being
        // relative refuses it, never what the cwd holds.
        ("relative home", asking("sh", "src"), Refusal::ProgramTree),
        (
            "absent home",
            asking("sh", "/no/such/home"),
            Refusal::ProgramTree,
        ),
        ("unread bootstrap", asking("sh", &home), Refusal::Identity),
    ];
    for (case, survey, cause) in cases {
        let reply = surveyed(&survey, &nameless);
        assert_eq!((case, reply), (case, Surveyed::Refused(cause)));
    }
}
