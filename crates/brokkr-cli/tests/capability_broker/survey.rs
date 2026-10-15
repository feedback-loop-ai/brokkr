//! `brokkr broker survey` (decision 0065 slice two, U7c0b) through the real
//! binary: one request on standard input, one reply line on standard
//! output, nonzero with the cause where the box refused or the request
//! did not read, and nothing on standard error.

use brokkr_protocol::broker::{Network, Reach, Survey, Surveyed};
use brokkr_protocol::hands::ServerProgram;

use super::*;

/// `binary`'s survey answering `said`, its input closed once written.
fn surveying(binary: &Path, said: &[u8]) -> Ran {
    let mut command = Command::new(binary);
    command.args(["broker", "survey"]).stdin(Stdio::piped());
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(said).unwrap();
    let Output {
        status,
        stdout,
        stderr,
    } = child.wait_with_output().unwrap();
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
    let ran = surveying(brokkr(), &line(&missing));
    let answered = (ran.code, ran.stdout.as_str(), ran.stderr.as_str());
    assert_eq!(answered, (Some(1), "{\"refused\":\"program-tree\"}\n", ""));
    // A seat that may write where the program lies: the box refuses before
    // anything is observed, on every host.
    let sh = ServerProgram::resolve("sh", &root.path).unwrap();
    let bin = sh.executable().parent().unwrap().to_path_buf();
    let in_reach = asking(&root, "sh", vec![bin]);
    let ran = surveying(brokkr(), &line(&in_reach));
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
            widened.to_string().into_bytes(),
            "{\"unread\":\"malformed\"}\n",
        ),
        (
            readable[..readable.len() - 2].to_vec(),
            "{\"unread\":\"truncated\"}\n",
        ),
        (vec![b' '; (1 << 20) + 1], "{\"unread\":\"oversized\"}\n"),
    ];
    for (said, reply) in unread {
        let ran = surveying(brokkr(), &said);
        let answered = (ran.code, ran.stdout.as_str(), ran.stderr.as_str());
        assert_eq!(answered, (Some(1), reply, ""));
    }
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
        let ran = surveying(brokkr(), &line(&survey));
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
