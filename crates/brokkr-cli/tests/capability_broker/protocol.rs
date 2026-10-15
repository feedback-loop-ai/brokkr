//! The filtered protocol through `serve` (decision 0065 slice two U6e,
//! U6f1b; MB3, MB4, CR3, SD3), on a real box: once the server has started
//! in its box it is initialized and its catalog listed within the
//! startup's absolute deadline, and then the harness is served on the
//! broker's own stdio, or, for a plan that retains its responses, refused
//! before any message reaches it. A server off its pinned version, one
//! lacking a granted tool, one that speaks no MCP, one that runs but lacks
//! a dependency inside its box, and one that never answers each end the
//! session with their own cause, latched in the ledger, after the one
//! lookup: distinct from the pre-secret refusals, which look nothing up,
//! and from an interpreter absent from the box, which starts nothing.

use std::io::Write;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{RecvTimeoutError, TryRecvError};
use std::time::{Duration, Instant};

use brokkr_protocol::broker::ledger::{Disposition, Failure, Invalid, Outcome};
use brokkr_protocol::secret::Guard;
use serde_json::{json, Value};

use super::ledger::{failed_with, sealed_ledger, Proved};
use super::observer::within;
use super::readiness::{boxed, Reads};
use super::{admitted, ended, killed, refused, set, skip, unservable, Refusal, Sealed, SPEAKS};
use super::{TALK_MAX, TOKEN};

/// What begins each piece of a line the broker relays from its server's
/// standard error, so that none reads as the broker's own (U6f2; operator
/// ruling 2026-10-14).
pub(super) const SERVER: &str = "MCP server stderr: ";

/// `lines` as the broker relays them from its server's standard error, each
/// labelled ([`SERVER`]).
pub(super) fn relayed(lines: &str) -> String {
    let labelled = lines
        .split_inclusive('\n')
        .map(|line| format!("{SERVER}{line}"));
    labelled.collect()
}

/// A sealed fixture whose server, started, runs `said` in the box's own
/// shell, and its plan's digest; none, declared skipped, where no box
/// stands.
pub(super) fn speaking(said: &str) -> Option<(Sealed, String)> {
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

/// A fixture `sealing` makes, and its plan's digest, sealed afresh until
/// the Opened its ledger first writes, which holds the plan's owner and
/// that digest, holds none of `values` as the final guard reads the
/// ledger's JSON, where the fixture's owner alone does not: a short value
/// a test binds, such as 0012, is held by a random digest often enough to
/// refuse the session at its start by chance (the controller's finding on
/// U6f1b's fourth run); one the fixed owner holds is refused every time,
/// as a test means it to be. None, declared skipped, where no box stands.
pub(super) fn unheld(
    sealing: impl Fn() -> Option<(Sealed, String)>,
    values: &[&str],
) -> Option<(Sealed, String)> {
    let root = tempfile::tempdir().unwrap();
    let store = root.path().canonicalize().unwrap().join("store.env");
    let bound: Vec<_> = values
        .iter()
        .map(|value| {
            brokkr_protocol::secret::store_set(&store, "VALUE", value).unwrap();
            let declared = ["VALUE".to_string()];
            brokkr_protocol::secret::resolve_bindings(&store, &declared).unwrap()
        })
        .collect();
    let guards: Vec<_> = bound.iter().map(|one| Guard::new(one)).collect();
    for _ in 0..8 {
        let (sealed, digest) = sealing()?;
        let owner = json!({"owner": sealed.authored()["owner"]}).to_string();
        let opened = json!({"owner": sealed.authored()["owner"], "plan": digest}).to_string();
        let by_chance =
            |guard: &Guard<'_>| guard.json(owner.as_bytes()) && !guard.json(opened.as_bytes());
        if !guards.iter().any(by_chance) {
            return Some((sealed, digest));
        }
    }
    panic!("eight sealed plans each held a bound value");
}

/// [`speaking`], its plan retaining nothing, so that a server that starts
/// is initialized before anything else (U6f1b).
pub(super) fn unretained(said: &str) -> Option<(Sealed, String)> {
    let (sealed, _) = speaking(said)?;
    let mut plan = sealed.plan();
    set(&mut plan, "/retained", json!(false));
    let digest = sealed.seal_bytes(plan.to_string().as_bytes());
    Some((sealed, digest))
}

/// `serve`'s answer, the lookups its store saw, and what its ledger proves.
type Served = ((Option<i32>, String), usize, Proved);

/// The fixture whose server runs `said`, its plan retaining nothing, and
/// `serve`'s answer for it, the lookups its store saw, and what its ledger
/// proves once nothing of its box is left ([`Served`]); none, declared
/// skipped, where no box stands.
fn served(said: &str) -> Option<(Sealed, Served)> {
    let (sealed, digest) = unretained(said)?;
    let reads = Reads::of(&sealed);
    let answer = sealed.serve(&digest);
    assert!(within(|| boxed(&sealed).is_empty()));
    let ran = (answer, reads.lookups(), sealed_ledger(&sealed));
    Some((sealed, ran))
}

/// A ledger closed Failed with `failure` and no call.
fn failed(failure: Failure) -> Proved {
    (vec![], Some(Disposition::Failed(failure)), None)
}

#[test]
fn a_retained_plan_is_refused_before_any_mcp_message_reaches_its_server() {
    // The plan retains its responses, which U8b builds, so `serve` refuses
    // with SD3's retention cause and never serves without it: once its
    // server has started, and before a single MCP message is exchanged
    // with it, initialize included (U6f1b). The server reports every line
    // it reads on its standard error, which the broker drains: it reads
    // none. The harness, whatever it says, hears nothing, and no call is
    // recorded.
    let heard = "while read -r line; do printf 'heard %s\\n' \"$line\" >&2; done\n";
    let Some((sealed, digest)) = speaking(heard) else {
        return;
    };
    let reads = Reads::on(sealed.path("store/secrets.env"));
    let ran = conversed(&sealed, &digest, &greeting(), 0);
    assert!(within(|| boxed(&sealed).is_empty()));
    let (code, stderr) = admitted();
    assert_eq!(ran, (code, vec![], String::new(), stderr));
    let incomplete = failed_with(Refusal::RetentionIncomplete);
    assert_eq!((reads.lookups(), sealed_ledger(&sealed)), (1, incomplete));
}

/// What the fixture's server says once it has started as [`SPEAKS`] says:
/// the bound value on its standard error, beside a character of two bytes,
/// before it answers ([`after`]).
fn answering() -> String {
    after(r#"printf 'server says %s é\n' "$DOCS_TOKEN" >&2"#)
}

/// A server that starts as [`SPEAKS`] says, and, once called, runs `probe`
/// in the box's own shell, then answers the broker's one forwarded call,
/// the broker's request 3, and waits for its input's end.
pub(super) fn after(probe: &str) -> String {
    let answer = r#"echo '{"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"resolved"}]}}'"#;
    format!("{SPEAKS}{probe}\n{answer}\nread -r line\n")
}

/// The harness's greeting, its listing as its request 2, and its call of
/// `resolve` as its request `c`.
pub(super) fn greeting() -> [Value; 4] {
    let hello = json!({"protocolVersion": "2025-06-18", "capabilities": {},
                       "clientInfo": {"name": "harness", "version": "1"}});
    let request = |id: Value, method: &str, params: Value| json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
    [
        request(json!(1), "initialize", hello),
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
        request(json!(2), "tools/list", json!({})),
        request(
            json!("c"),
            "tools/call",
            json!({"name": "resolve", "arguments": {}}),
        ),
    ]
}

/// `serve`'s answer for the plan `sealed` sealed at `digest`, started as a
/// harness starts it, which says `frames` and reads `answers` frames
/// before it ends its input ([`talking`]).
pub(super) fn conversed(sealed: &Sealed, digest: &str, frames: &[Value], answers: usize) -> Ran {
    talking(sealed, digest, frames, answers).ended()
}

/// A broker's exit, the frames its harness read, whatever it wrote after
/// them, and its standard error.
pub(super) type Ran = (Option<i32>, Vec<Value>, String, String);

/// `serve` of the plan `sealed` sealed at `digest`, a fresh lifetime of it,
/// started as a harness starts it, which has said `frames` and read
/// `answers` frames and still holds its input open: the broker, that
/// input, its standard output and its standard error, each read apart from
/// its start so that neither fills ([`super::Apart`]), how much of its
/// output the frames read took, those frames, and how long each wait for
/// it may take.
pub(super) struct Talk {
    child: Child,
    input: Option<std::process::ChildStdin>,
    output: super::Apart,
    errors: Option<super::Apart>,
    taken: usize,
    heard: Vec<Value>,
    max: Duration,
}

/// `serve` of the plan `sealed` sealed at `digest`, a fresh lifetime of it,
/// as a harness starts it, its stdio not yet chosen.
pub(super) fn harnessing(sealed: &Sealed, digest: &str) -> std::process::Command {
    std::fs::remove_file(super::ledger::of(&sealed.locator)).ok();
    let locator = sealed.locator.to_str().unwrap();
    let args = [
        "broker",
        "serve",
        "--plan",
        locator,
        "--plan-digest",
        digest,
    ];
    let mut command = sealed.root.harnessed(&args);
    command.env("HOME", &sealed.home);
    command
}

pub(super) fn talking(sealed: &Sealed, digest: &str, frames: &[Value], answers: usize) -> Talk {
    talking_beside(sealed, digest, (frames, answers), Stdio::piped())
}

/// [`talking`], the broker's standard error `stderr`.
pub(super) fn talking_beside(
    sealed: &Sealed,
    digest: &str,
    (frames, answers): (&[Value], usize),
    stderr: Stdio,
) -> Talk {
    let mut command = harnessing(sealed, digest);
    command.stdin(Stdio::piped()).stdout(Stdio::piped());
    let child = command.stderr(stderr).spawn().unwrap();
    Talk::over(child, (frames, answers), TALK_MAX)
}

impl Talk {
    /// `child`, its input and output piped and its standard error piped or
    /// not, told `frames` and heard until it has said `answers` frames or
    /// its output has ended, each wait for it within `max`.
    pub(super) fn over(
        mut child: Child,
        (frames, answers): (&[Value], usize),
        max: Duration,
    ) -> Talk {
        let output = super::apart(child.stdout.take().unwrap());
        let errors = child.stderr.take().map(super::apart);
        let mut input = child.stdin.take().unwrap();
        for frame in frames {
            writeln!(input, "{frame}").unwrap();
        }
        let mut talk = Talk {
            child,
            input: Some(input),
            output,
            errors,
            taken: 0,
            heard: Vec::new(),
            max,
        };
        talk.heard_to(answers);
        talk
    }

    /// The broker's pid.
    pub(super) fn id(&self) -> u32 {
        self.child.id()
    }

    /// The harness says `frame`, then reads the broker's next frame, where
    /// its output has not ended.
    pub(super) fn asked(&mut self, frame: &Value) {
        writeln!(self.input.as_mut().unwrap(), "{frame}").ok();
        self.heard_to(self.heard.len() + 1);
    }

    /// The broker's frames read until `answers` are, or its output has
    /// ended, within an absolute deadline [`Talk::max`] from now, read
    /// again on every look, however its bytes arrive: a broker that says
    /// too little, or bytes without a newline, fails the test there
    /// ([`Talk::failed`]).
    fn heard_to(&mut self, answers: usize) {
        let until = Instant::now() + self.max;
        while self.heard.len() < answers {
            if Instant::now() >= until {
                self.failed("the broker said too little within its bound");
            }
            let ended = !matches!(self.output.1.try_recv(), Err(TryRecvError::Empty));
            let line = {
                let said = self.output.0.lock().unwrap();
                let unread = &said[self.taken..];
                let end = unread.iter().position(|byte| *byte == b'\n');
                end.map(|end| unread[..=end].to_vec())
            };
            match line {
                Some(line) => {
                    self.taken += line.len();
                    self.heard.push(serde_json::from_slice(&line).unwrap());
                }
                None if ended => return,
                None => std::thread::sleep(Duration::from_millis(10)),
            }
        }
    }

    /// What the broker has said on its standard error so far.
    fn errors(&self) -> String {
        let said = self
            .errors
            .as_ref()
            .map(|(said, _)| said.lock().unwrap().clone());
        String::from_utf8_lossy(&said.unwrap_or_default()).into_owned()
    }

    /// The test failed for `why`, naming the broker's status, it and its
    /// children killed first where it still runs, and its standard error
    /// so far (the review of U6f1b's seventh run).
    fn failed(&mut self, why: &str) -> ! {
        let status = match self.child.try_wait().unwrap() {
            Some(status) => status,
            None => {
                killed(self.child.id());
                self.child.wait().unwrap()
            }
        };
        panic!("{why}: {status:?}\n{}", self.errors())
    }

    /// The harness's input ended, and the broker's answer once it has
    /// ended ([`Ran`]): its exit, then the end of its output and of its
    /// standard error, each within an absolute deadline [`Talk::max`] from
    /// the input's end, so that a broker that never ends, or anything it
    /// left behind holding either stream open, fails the test.
    pub(super) fn ended(mut self) -> Ran {
        drop(self.input.take());
        let until = Instant::now() + self.max;
        let status = loop {
            match self.child.try_wait().unwrap() {
                Some(status) => break status,
                None if Instant::now() >= until => {
                    self.failed("the broker did not end within its bound")
                }
                None => std::thread::sleep(Duration::from_millis(10)),
            }
        };
        let left = || until.saturating_duration_since(Instant::now());
        let mut streams = std::iter::once(&self.output).chain(&self.errors);
        let timeout = Err(RecvTimeoutError::Timeout);
        if streams.any(|(_, ending)| ending.recv_timeout(left()) == timeout) {
            self.failed("the broker's output did not end within its bound");
        }
        let said = self.output.0.lock().unwrap()[self.taken..].to_vec();
        let rest = String::from_utf8(said).unwrap();
        (
            status.code(),
            std::mem::take(&mut self.heard),
            rest,
            self.errors(),
        )
    }
}

/// A shell in a process group of its own running `script` once it has said
/// a line on its standard error, its standard streams piped.
fn standing_in(script: &str) -> Child {
    let mut command = Command::new("sh");
    command.args(["-c", &format!("echo said >&2; {script}")]);
    command.stdin(Stdio::piped()).stdout(Stdio::piped());
    command
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()
        .unwrap()
}

/// What `waited` panicked with, a wait of a test's bounded within a second,
/// on the process group `group`, which is then killed whole, and whether it
/// panicked within ten seconds.
fn failing(group: u32, waited: impl FnOnce()) -> (String, bool) {
    let begun = Instant::now();
    let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(waited));
    let within = begun.elapsed() < Duration::from_secs(10);
    let group = format!("-{group}");
    let mut kill = Command::new("kill");
    kill.args(["-KILL", "--", &group]).stderr(Stdio::null());
    kill.status().ok();
    let said = panicked.unwrap_err().downcast::<String>().unwrap();
    (*said, within)
}

#[test]
fn every_wait_of_a_public_test_fails_within_its_bound_naming_the_status_and_stderr() {
    // A harness hearing bytes and never a newline, a broker that never
    // ends, and one that ends while what it left behind holds its output
    // or its standard error open, each fails its test within its bound,
    // read on every look, naming the status and the standard error, or
    // the file its starter sent that to (the review of U6f1b's seventh run).
    // Each shell runs no child of its own where it is killed, so its status
    // is its own kill's.
    let bound = Duration::from_secs(1);
    let killed = "ExitStatus(unix_wait_status(9))";
    let unheard = format!("the broker said too little within its bound: {killed}");
    let unended = format!("the broker did not end within its bound: {killed}");
    let held = "the broker's output did not end within its bound: ExitStatus(unix_wait_status(0))";
    let left = |held: &str| format!("sleep 30 {held}/dev/null </dev/null & exit 0");
    let cases = [
        (
            "bytes",
            "while :; do printf x; done".to_string(),
            1,
            unheard,
        ),
        ("unended", "exec sleep 30".to_string(), 0, unended),
        ("stdout", left("2>"), 0, held.to_string()),
        ("stderr", left(">"), 0, held.to_string()),
    ];
    for (case, script, answers, why) in cases {
        let child = standing_in(&script);
        let group = child.id();
        let failed = failing(group, || {
            drop(Talk::over(child, (&[], answers), bound).ended())
        });
        let expected = (format!("{why}\nsaid\n"), true);
        assert_eq!((case, failed), (case, expected));
    }
    let root = tempfile::tempdir().unwrap();
    let apart = root.path().join("broker.err");
    let child = standing_in(&format!("echo told >'{}'; exec sleep 30", apart.display()));
    let group = child.id();
    let failed = failing(group, || {
        drop(super::bounded_by(child, b"", (bound, Some(&apart))))
    });
    let expected = format!("the broker did not end: {killed}\nsaid\ntold\n");
    assert_eq!(failed, (expected, true));
}

#[test]
fn a_nonretaining_plan_serves_its_harness_masked_and_closes_clean() {
    // The plan retains nothing, so once its server has initialized the
    // harness is served on the broker's own stdio: greeted and listed
    // here, its granted call forwarded and answered under its own id, and
    // the server's own standard error, which says the bound value, drained
    // and masked on its raw bytes before it is decoded, its two-byte
    // character whole. When the harness ends its input, the session closes
    // Clean with its one call, and no byte anywhere holds the value.
    let Some((sealed, digest)) = unretained(&answering()) else {
        return;
    };
    let reads = Reads::on(sealed.path("store/secrets.env"));
    let ran = conversed(&sealed, &digest, &greeting(), 3);
    assert!(within(|| boxed(&sealed).is_empty()));
    let server = json!({"name": "cap-library-docs", "version": "1.4.2"});
    let schema = json!({"type": "object"});
    let tools = json!([{"name": "resolve", "inputSchema": schema},
                       {"name": "read", "inputSchema": schema}]);
    let content = json!({"content": [{"type": "text", "text": "resolved"}]});
    let answers = vec![
        json!({"jsonrpc": "2.0", "id": 1, "result": {"protocolVersion": "2025-06-18",
               "capabilities": {"tools": {}}, "serverInfo": server}}),
        json!({"jsonrpc": "2.0", "id": 2, "result": {"tools": tools}}),
        json!({"jsonrpc": "2.0", "id": "c", "result": content}),
    ];
    let said = relayed("server says [secret:DOCS_TOKEN] é\n");
    assert_eq!(ran, (Some(0), answers, String::new(), said));
    let one = vec![("resolve".to_string(), Some(Outcome::Succeeded))];
    let clean = (one, Some(Disposition::Clean), None);
    assert_eq!((reads.lookups(), sealed_ledger(&sealed)), (1, clean));
    let ledger = std::fs::read(super::ledger::of(&sealed.locator)).unwrap();
    assert!(!String::from_utf8_lossy(&ledger).contains(TOKEN));
}

#[test]
fn a_servers_error_line_in_a_clean_session_is_relayed_as_the_servers_never_the_brokers() {
    // The server says `error: ` on its standard error and serves its call
    // all the same: the broker relays that line labelled the server's, so
    // it never reads as the broker's own diagnostic, exits 0 and closes
    // Clean. The exit status, not the standard error, says how the session
    // ended (U6f2; operator ruling 2026-10-14).
    let Some((sealed, digest)) = unretained(&after("echo 'error: quota nearly spent' >&2")) else {
        return;
    };
    let ran = conversed(&sealed, &digest, &greeting(), 3);
    let said = format!("{SERVER}error: quota nearly spent\n");
    assert_eq!((ran.0, ran.1.len(), ran.3), (Some(0), 3, said));
    let one = vec![("resolve".to_string(), Some(Outcome::Succeeded))];
    let clean = (one, Some(Disposition::Clean), None);
    assert_eq!(sealed_ledger(&sealed), clean);
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
        let Some((_, ran)) = served(&said) else {
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
    // (`handoff`). What its shell said of it on its standard error, drained
    // since its start, is written, masked, before the cause, as on every
    // ending (U6f1b).
    let dependency = "/opt/docs/lib/helper --serve\n";
    let corrupt = "read -r line\necho 'this is no frame'\nread -r line\n";
    let missing = ": 2: /opt/docs/lib/helper: not found\n";
    for (case, said, says) in [
        ("dependency", dependency, missing),
        ("corrupt", corrupt, ""),
    ] {
        let Some((sealed, ran)) = served(said) else {
            return;
        };
        let server = sealed.path("opt/docs/bin/docs-mcp");
        let told = match says.is_empty() {
            true => String::new(),
            false => relayed(&format!("{}{says}", server.display())),
        };
        let (code, cause) = ended(Failure::Protocol);
        let protocol = ((code, told + &cause), 1, failed(Failure::Protocol));
        assert_eq!((case, ran), (case, protocol));
    }
}

#[test]
fn a_binding_the_startups_closure_would_hold_ends_the_session_masked_and_unclosed() {
    // The store binds `closed`, and the server speaks no MCP: the ledger
    // is given the binding as soon as the handoff looks it up, so the
    // Closed the protocol cause would close with, which holds that word,
    // is never appended, and `serve` ends with the masking cause (MB4,
    // CR2; operator ruling 2026-10-13). Only the Opened verifies.
    let corrupt = "read -r line\necho 'this is no frame'\nread -r line\n";
    let Some((sealed, digest)) = unretained(corrupt) else {
        return;
    };
    let store = sealed.path("store/secrets.env");
    std::fs::write(&store, "DOCS_TOKEN=closed\n").unwrap();
    let reads = Reads::on(store);
    let answer = sealed.serve(&digest);
    assert!(within(|| boxed(&sealed).is_empty()));
    let unclosed = (vec![], None, Some(Invalid::Lifecycle));
    assert_eq!(
        (answer, reads.lookups(), sealed_ledger(&sealed)),
        (ended(Failure::Masking), 1, unclosed)
    );
}

/// `serve` of a plan retaining nothing whose server says `said` and whose
/// store holds `store`, the plan declaring its names and granting `tool`
/// beside the fixture's own: its exit and its whole standard error, where
/// no store value stands; and what its ledger proves.
fn last_line(said: &str, store: &str, tool: &str) -> Option<((Option<i32>, String), Proved)> {
    let (names, values): (Vec<&str>, Vec<&str>) = store
        .lines()
        .filter_map(|line| line.split_once('='))
        .unzip();
    let sealing = || {
        let (sealed, _) = speaking(said)?;
        std::fs::write(sealed.path("store/secrets.env"), store).unwrap();
        let mut plan = sealed.plan();
        set(&mut plan, "/retained", json!(false));
        set(&mut plan, "/secrets", json!(names));
        set(&mut plan, "/tools", json!(["resolve", "read", tool]));
        let digest = sealed.seal_bytes(plan.to_string().as_bytes());
        Some((sealed, digest))
    };
    let (sealed, digest) = unheld(sealing, &values)?;
    let answer = sealed.serve(&digest);
    assert!(within(|| boxed(&sealed).is_empty()));
    let unsaid = values.iter().find(|value| answer.1.contains(**value));
    assert_eq!(unsaid, None, "{}", answer.1);
    Some((answer, sealed_ledger(&sealed)))
}

#[test]
fn the_brokers_last_line_is_masked_whole_or_replaced_while_its_bindings_are_held() {
    // The broker's own last line is rendered whole, `error: ` included, and
    // passes the one voice its server's lines pass while the bindings are
    // still held, once the ledger has closed (U6f1b; MB4): a bound value is
    // masked wherever it stands in it, the prefix's word or the cause's;
    // one it holds only by its value, as a granted tool's digits are, which
    // masking cannot remove, is replaced by a fixed line; and where masking
    // that one leaves a bound value too, as a name with a digit does, no
    // line is written at all, nor the closure, which holds that value too,
    // and the exit code alone says it failed.
    let corrupt = "read -r line\necho 'this is no frame'\nread -r line\n";
    let cases = [
        (
            "error",
            corrupt,
            "DOCS_TOKEN=error\n",
            "[secret:DOCS_TOKEN]: MCP server protocol is invalid\n",
        ),
        (
            "cause",
            corrupt,
            "DOCS_TOKEN=invalid\n",
            "error: MCP server protocol is [secret:DOCS_TOKEN]\n",
        ),
        (
            "digits",
            SPEAKS,
            "DOCS_TOKEN=04242424242\n",
            "error: MCP broker session failed and its cause cannot be safely reported\n",
        ),
        ("none", corrupt, "DOCS_TOKEN=0002\nKEY2=error\n", ""),
    ];
    for (case, said, store, line) in cases {
        let Some(ran) = last_line(said, store, "v4242424242") else {
            return;
        };
        let proved = match case {
            "none" => (vec![], None, Some(Invalid::Lifecycle)),
            _ => failed(Failure::Protocol),
        };
        assert_eq!((case, ran), (case, ((Some(1), line.to_string()), proved)));
    }
}

#[test]
fn a_server_that_never_answers_ends_at_the_startup_deadline() {
    // It reads every request and answers none, so its box stands until
    // the startup's absolute deadline, which admission and readiness have
    // already spent from: thirty seconds from `serve`'s start, and no more
    // than a slice after.
    let Some((sealed, digest)) = unretained("while read -r line; do :; done\n") else {
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
