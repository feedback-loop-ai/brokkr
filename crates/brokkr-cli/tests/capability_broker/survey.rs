//! `brokkr broker survey` (decision 0065 slice two, U7c0b, U7c) through the
//! real binary: one request line on standard input, one reply line on
//! standard output, nonzero with the cause where the box refused or the
//! request did not read, and nothing on standard error. Once its request
//! is read, the end of its input kills it with its whole group, so it
//! ends with the engine that holds that input (operator ruling
//! 2026-10-16).

use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::os::unix::process::{CommandExt, ExitStatusExt};

use brokkr_protocol::broker::{Network, Reach, Survey, Surveyed};
use brokkr_protocol::hands::ServerProgram;
use rustix::io::Errno;
use rustix::process::{kill_process_group, test_kill_process_group, Pid, Signal};

use super::*;

/// How a test's survey input is left once its bytes are written.
#[derive(Clone, Copy)]
enum Input {
    /// Held open until the survey has ended, as the engine holds it.
    Held,
    /// Closed, as a request that ends before its newline needs.
    Closed,
}

/// `binary`'s survey answering `said`, in a process group of its own as
/// the engine starts it, its input left as `input` says.
fn surveying(binary: &Path, said: &[u8], input: Input) -> Ran {
    let mut command = Command::new(binary);
    command.args(["broker", "survey"]).process_group(0);
    surveying_as(command, said, input)
}

/// What `command`, a survey, answers `said`, its input left as `input`
/// says.
fn surveying_as(mut command: Command, said: &[u8], input: Input) -> Ran {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut written = child.stdin.take().unwrap();
    written.write_all(said).unwrap();
    let held = match input {
        Input::Held => Some(written),
        Input::Closed => {
            drop(written);
            None
        }
    };
    let Output {
        status,
        stdout,
        stderr,
    } = child.wait_with_output().unwrap();
    drop(held);
    let text = |bytes| String::from_utf8(bytes).unwrap();
    Ran {
        code: status.code(),
        stdout: text(stdout),
        stderr: text(stderr),
    }
}

/// A request to survey `launch` from `root`'s HOME, the seat writing
/// `writable`, this process's uid the one sealed writer.
fn asking(root: &Root, launch: &str, writable: Vec<PathBuf>) -> Survey {
    Survey {
        launch: launch.into(),
        home: root.path.clone(),
        reach: Reach {
            writable,
            readable: Vec::new(),
        },
        network: Network::Isolated,
        writers: vec![euid()],
    }
}

/// `survey`'s wire form, one line.
fn line(survey: &Survey) -> Vec<u8> {
    let mut line = serde_json::to_vec(survey).unwrap();
    line.push(b'\n');
    line
}

/// The reply `text` holds, as one line and nothing else.
fn reply(text: &str) -> Surveyed {
    let (line, rest) = text.split_once('\n').unwrap();
    assert_eq!(rest, "");
    serde_json::from_str(line).unwrap()
}

#[test]
fn a_box_refused_or_a_program_missing_replies_its_cause_alone_and_fails() {
    let root = Root::new();
    let missing = asking(&root, "no-such-mcp-server", vec![root.path.join("work")]);
    let ran = surveying(brokkr(), &line(&missing), Input::Held);
    let answered = (ran.code, ran.stdout.as_str(), ran.stderr.as_str());
    assert_eq!(answered, (Some(1), "{\"refused\":\"program-tree\"}\n", ""));
    // A seat that may write where the program lies: the box refuses before
    // anything is observed, on every host.
    let sh = ServerProgram::resolve("sh", &root.path).unwrap();
    let bin = sh.executable().parent().unwrap().to_path_buf();
    let in_reach = asking(&root, "sh", vec![bin]);
    let ran = surveying(brokkr(), &line(&in_reach), Input::Held);
    assert_eq!((ran.code, ran.stderr.as_str()), (Some(1), ""));
    assert_eq!(
        reply(&ran.stdout),
        Surveyed::Refused(Refusal::LaunchInReach)
    );
}

#[test]
fn a_request_it_cannot_read_replies_why_alone_and_fails() {
    let root = Root::new();
    let readable = line(&asking(&root, "sh", Vec::new()));
    let mut widened: Value = serde_json::from_slice(&readable).unwrap();
    widened["plan"] = json!("/p");
    let unread = [
        (
            format!("{widened}\n").into_bytes(),
            Input::Held,
            "{\"unread\":\"malformed\"}\n",
        ),
        (
            readable[..readable.len() - 1].to_vec(),
            Input::Closed,
            "{\"unread\":\"truncated\"}\n",
        ),
        (
            vec![b' '; 1 << 20],
            Input::Held,
            "{\"unread\":\"oversized\"}\n",
        ),
    ];
    for (said, input, reply) in unread {
        let ran = surveying(brokkr(), &said, input);
        let answered = (ran.code, ran.stdout.as_str(), ran.stderr.as_str());
        assert_eq!(answered, (Some(1), reply, ""));
    }
}

/// A survey that leads no process group of its own arms no watcher that
/// could kill its caller's: it refuses identity, and this test and its
/// group live on.
#[test]
fn a_survey_leading_no_group_of_its_own_refuses_identity() {
    let root = Root::new();
    let missing = asking(&root, "no-such-mcp-server", Vec::new());
    let mut command = Command::new(brokkr());
    command.args(["broker", "survey"]);
    // Held until it ends: were a watcher armed, the input's end could
    // only kill this test's own group.
    let ran = surveying_as(command, &line(&missing), Input::Held);
    let answered = (ran.code, ran.stdout.as_str(), ran.stderr.as_str());
    assert_eq!(answered, (Some(1), "{\"refused\":\"identity\"}\n", ""));
}

/// The two ends of a stream whose second takes no more, nothing reading
/// the first: a survey's reply written there waits.
fn stopped_output() -> (UnixStream, UnixStream) {
    let (ours, theirs) = UnixStream::pair().unwrap();
    theirs.set_nonblocking(true).unwrap();
    let bytes = [0; 4096];
    for size in [bytes.len(), 1] {
        while (&theirs).write(&bytes[..size]).is_ok() {}
    }
    theirs.set_nonblocking(false).unwrap();
    (ours, theirs)
}

/// Once its request is read, a survey ends with its input, whatever it is
/// doing: one whose reply waits on an output nobody reads is still there
/// half a second on, and the input's end kills it within a few seconds,
/// leaving nothing of its group.
#[test]
fn the_end_of_its_input_after_the_request_kills_the_survey_and_its_group() {
    let root = Root::new();
    let (_ours, theirs) = stopped_output();
    let mut child = Command::new(brokkr())
        .args(["broker", "survey"])
        .process_group(0)
        .stdin(Stdio::piped())
        .stdout(Stdio::from(OwnedFd::from(theirs)))
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let group = Pid::from_child(&child);
    let mut input = child.stdin.take().unwrap();
    let missing = asking(&root, "no-such-mcp-server", Vec::new());
    input.write_all(&line(&missing)).unwrap();
    std::thread::sleep(Duration::from_millis(500));
    assert_eq!(child.try_wait().unwrap(), None, "its reply waits");
    drop(input);
    let until = Instant::now() + Duration::from_secs(10);
    let ended = loop {
        let status = child.try_wait().unwrap();
        if status.is_some() || Instant::now() >= until {
            break status;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    if ended.is_none() {
        kill_process_group(group, Signal::KILL).ok();
        child.wait().unwrap();
    }
    assert_eq!(ended.and_then(|status| status.signal()), Some(9));
    assert_eq!(test_kill_process_group(group), Err(Errno::SRCH));
}

#[cfg(target_os = "linux")]
#[test]
fn the_survey_replies_the_program_bootstrap_and_sources_provisioning_observes() {
    use brokkr_protocol::broker::{Bootstrap, Provisioned, Tree};
    use brokkr_protocol::hands::{ServerBox, ServerProfile};
    if let Some(reason) = unservable() {
        return skip(&reason);
    }
    let root = Root::new();
    let bytes = std::fs::read(brokkr()).unwrap();
    // A system entry, its own tree, and a package planted under its root.
    let docs = root.path.join("opt/docs");
    let entry = plant(&docs.join("bin/docs-mcp"));
    let package = Tree::Package { root: docs };
    for (launch, tree) in [("sh", Tree::System {}), (entry.to_str().unwrap(), package)] {
        let survey = asking(&root, launch, vec![root.path.join("work")]);
        let ran = surveying(brokkr(), &line(&survey), Input::Held);
        assert_eq!(
            (launch, ran.code, ran.stderr.as_str()),
            (launch, Some(0), "")
        );
        let program = ServerProgram::resolve(launch, &root.path).unwrap();
        let profile = ServerProfile {
            reach: &survey.reach,
            network: &survey.network,
            bootstrap: brokkr(),
            writers: &survey.writers,
        };
        let expected = Provisioned {
            executable: program.executable().to_path_buf(),
            tree,
            bootstrap: Bootstrap {
                path: bootstrap(),
                digest: brokkr_core::canonical::sha256_bytes(&bytes),
            },
            sources: ServerBox::provision(&program, &profile).unwrap(),
        };
        assert_eq!(reply(&ran.stdout), Surveyed::Provisioned(expected));
    }
}
