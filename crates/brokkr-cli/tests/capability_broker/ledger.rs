//! The broker's private ledger (decision 0065 slice two U6d; CR3, CC2,
//! MB3, MB4): its writer, its reader over a settled ledger, and the one
//! lifetime the real binary gives a bound plan. The writer records exactly
//! D7's order, synced before the broker acts, and nothing after a record it
//! could not make; the reader keeps only the prefix it verifies, with the
//! cause where it stops; a bound plan's ledger is created once, before its
//! box, and closed Failed with the session's first cause, while an unbound
//! plan has none. The broker writes no journal.

use std::cell::RefCell;
use std::ffi::OsString;
use std::io::BufRead;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use brokkr_protocol::broker::ledger::{
    recover, Denial, Disposition, Failure, Invalid, Ledger, Lifecycle, Outcome, Unrecorded,
};
use brokkr_protocol::broker::Owner;
use serde_json::{json, Value};

use super::{bindable, binds, past_prepare, refused, set, skip, unservable};
use super::{Refusal, Root, Sealed};

/// The ledger of the plan at `locator`: beside it, named after it.
pub(super) fn of(locator: &Path) -> PathBuf {
    let mut ledger = OsString::from(locator);
    ledger.push(".ledger");
    PathBuf::from(ledger)
}

/// The plan digest every writer test opens its ledger for.
const PLAN: &str = "5ea1ed0000000000000000000000000000000000000000000000000000000000";

/// The owner every writer test opens its ledger for.
fn owner() -> Owner {
    Owner {
        repo: "a".repeat(64),
        run: "run-1".into(),
        effect: super::EFFECT.into(),
        attempt: super::ATTEMPT.into(),
        site: "research".into(),
        instance: "research#0".into(),
    }
}

/// How the `nth` append, counted from one, fails: nothing of it written,
/// half of it written, or all of it written and synced but its
/// acknowledgement lost.
#[derive(Clone, Copy, Debug)]
enum Fault {
    Unwritten,
    Torn,
    Unacknowledged,
}

/// What a test's ledger made durable, and how many appends were asked of
/// it.
#[derive(Default)]
struct Disk {
    bytes: Vec<u8>,
    appends: usize,
}

/// A ledger over memory for the plan [`PLAN`] of [`owner`], its `fault`
/// at its append, and what it made durable.
fn opened(fault: Option<(usize, Fault)>) -> (Result<Ledger, Unrecorded>, Rc<RefCell<Disk>>) {
    let disk = Rc::new(RefCell::new(Disk::default()));
    let written = Rc::clone(&disk);
    let append = move |line: &[u8]| {
        let mut disk = written.borrow_mut();
        disk.appends += 1;
        let kept = match fault {
            Some((nth, Fault::Unwritten)) if nth == disk.appends => 0,
            Some((nth, Fault::Torn)) if nth == disk.appends => line.len() / 2,
            _ => line.len(),
        };
        disk.bytes.extend_from_slice(&line[..kept]);
        match fault.is_some_and(|(nth, _)| nth == disk.appends) {
            true => Err(std::io::Error::other("fault")),
            false => Ok(()),
        }
    };
    (Ledger::open_with(append, &owner(), PLAN), disk)
}

/// A ledger over memory that records everything.
fn healthy() -> (Ledger, Rc<RefCell<Disk>>) {
    let (ledger, disk) = opened(None);
    (ledger.unwrap(), disk)
}

/// What the durable `bytes` prove for [`PLAN`] and [`owner`]: each call's
/// tool and outcome, the closure, and where verification stopped.
pub(super) type Proved = (
    Vec<(String, Option<Outcome>)>,
    Option<Disposition>,
    Option<Invalid>,
);

fn proved(bytes: &[u8]) -> Proved {
    let (lifecycle, invalid) = recover(bytes, &owner(), PLAN);
    flat(&lifecycle, invalid)
}

fn flat(lifecycle: &Lifecycle, invalid: Option<Invalid>) -> Proved {
    (
        lifecycle.calls().to_vec(),
        lifecycle.closed().cloned(),
        invalid,
    )
}

/// One accepted call to `tool`, ended with `outcome`.
fn call(ledger: &mut Ledger, tool: &str, outcome: Outcome) {
    let begun = ledger.room().unwrap().begin(tool).unwrap();
    ledger.end(begun, outcome).unwrap();
}

/// `tool`'s call ended with `outcome`, as the reader projects it.
fn ended(tool: &str, outcome: Outcome) -> (String, Option<Outcome>) {
    (tool.to_string(), Some(outcome))
}

#[test]
fn each_ledger_cause_reads_in_cr3s_words() {
    let texts: [(&dyn std::error::Error, &str); 16] = [
        (&Unrecorded::Reopened, "broker plan has already been opened"),
        (&Unrecorded::Record, "broker call could not be recorded"),
        (
            &Unrecorded::Limit,
            "broker ledger exceeds the attempt limit",
        ),
        (
            &Failure::Version,
            "MCP server version does not match the dialect's pinned version",
        ),
        (&Failure::Protocol, "MCP server protocol is invalid"),
        (
            &Failure::Response,
            "MCP server response exceeds the broker limit",
        ),
        (&Failure::Timeout, "MCP server call timed out"),
        (&Failure::Masking, "MCP response cannot be safely masked"),
        (&Failure::Limit, "broker ledger exceeds the attempt limit"),
        (
            &Failure::Refused(Refusal::Establishment),
            "MCP server box could not be established",
        ),
        (&Invalid::Lifecycle, "broker ledger lifecycle is invalid"),
        (
            &Invalid::Owner,
            "broker ledger owner does not match this attempt",
        ),
        (
            &Invalid::Conflict,
            "broker ledger contains a conflicting duplicate",
        ),
        (&Invalid::Partial, "broker ledger ends in a partial record"),
        (&Invalid::Malformed, "broker ledger record is malformed"),
        (&Invalid::Gap, "broker ledger sequence is not contiguous"),
    ];
    for (cause, text) in texts {
        assert_eq!(cause.to_string(), text);
    }
}

#[test]
fn a_healthy_session_with_no_call_is_opened_then_closed_clean() {
    let (ledger, disk) = healthy();
    assert_eq!(ledger.close(), Ok(Disposition::Clean));
    let bytes = disk.borrow().bytes.clone();
    // Exactly Opened, naming the owner and the plan, then Closed: every
    // record one closed object on its own line, numbered from one.
    let opened = json!({"record": "opened", "seq": 1, "owner": {"repo": "a".repeat(64),
        "run": "run-1", "effect": super::EFFECT, "attempt": super::ATTEMPT,
        "site": "research", "instance": "research#0"}, "plan": PLAN});
    let closed = json!({"record": "closed", "seq": 2, "disposition": "clean"});
    let lines: Vec<Value> = bytes
        .lines()
        .map(|line| serde_json::from_str(&line.unwrap()).unwrap())
        .collect();
    assert_eq!(lines, [opened, closed]);
    assert_eq!(proved(&bytes), (Vec::new(), Some(Disposition::Clean), None));
}

#[test]
fn the_first_fatal_cause_survives_later_causes_and_the_end_of_input() {
    // A pinned version that does not match, before any call; later causes
    // and the end of input do not replace it.
    let (mut ledger, disk) = healthy();
    ledger.fail(Failure::Version);
    ledger.fail(Failure::Protocol);
    let failed = Disposition::Failed(Failure::Version);
    assert_eq!(ledger.close(), Ok(failed.clone()));
    let bytes = disk.borrow().bytes.clone();
    assert_eq!(proved(&bytes), (Vec::new(), Some(failed), None));
    // A fatal protocol failure after one call that ended.
    let (mut ledger, disk) = healthy();
    call(&mut ledger, "resolve", Outcome::Succeeded);
    ledger.fail(Failure::Protocol);
    ledger.fail(Failure::Limit);
    let failed = Disposition::Failed(Failure::Protocol);
    assert_eq!(ledger.close(), Ok(failed.clone()));
    let calls = vec![ended("resolve", Outcome::Succeeded)];
    assert_eq!(proved(&disk.borrow().bytes), (calls, Some(failed), None));
}

#[test]
fn each_served_cause_closes_a_durable_ledger_first_and_is_never_replaced() {
    let root = Root::new();
    let dir = std::fs::File::open(&root.path).unwrap();
    // Each cause, its name on the wire, and a distinct later cause.
    let causes = [
        (Failure::Response, "response", Failure::Timeout),
        (Failure::Timeout, "timeout", Failure::Masking),
        (Failure::Masking, "masking", Failure::Response),
    ];
    for (cause, name, later) in causes {
        // Latched after a call that ended, through the file the broker
        // writes: the later cause and the end of input do not replace it,
        // and the settled file proves exactly it.
        let plan = OsString::from(format!("{name}.json"));
        let mut ledger = Ledger::open(&dir, &plan, &owner(), PLAN).unwrap();
        call(&mut ledger, "resolve", Outcome::Failed);
        ledger.fail(cause.clone());
        ledger.fail(later.clone());
        let failed = Disposition::Failed(cause.clone());
        assert_eq!((name, ledger.close()), (name, Ok(failed.clone())));
        let bytes = std::fs::read(of(&root.path.join(&plan))).unwrap();
        let closed: Value = serde_json::from_str(&bytes.lines().last().unwrap().unwrap()).unwrap();
        let wire = json!({"record": "closed", "seq": 4, "disposition": {"failed": name}});
        assert_eq!((name, closed), (name, wire));
        let calls = vec![ended("resolve", Outcome::Failed)];
        assert_eq!((name, proved(&bytes)), (name, (calls, Some(failed), None)));
        // Latched after a first cause, it does not replace that one.
        let (mut ledger, disk) = healthy();
        ledger.fail(later.clone());
        ledger.fail(cause);
        let first = Disposition::Failed(later);
        assert_eq!((name, ledger.close()), (name, Ok(first.clone())));
        let recovered = proved(&disk.borrow().bytes);
        assert_eq!((name, recovered), (name, (vec![], Some(first), None)));
    }
}

#[test]
fn tool_errors_and_local_denials_are_calls_and_leave_the_session_clean() {
    let (mut ledger, disk) = healthy();
    let outcomes = [
        ("resolve", Outcome::Failed),
        ("admin", Outcome::Refused(Denial::Ungranted)),
        ("read", Outcome::Refused(Denial::Active)),
        ("read", Outcome::Refused(Denial::Budget)),
    ];
    for (tool, outcome) in outcomes {
        call(&mut ledger, tool, outcome);
    }
    assert_eq!(ledger.close(), Ok(Disposition::Clean));
    let bytes = disk.borrow().bytes.clone();
    let calls = outcomes
        .map(|(tool, outcome)| ended(tool, outcome))
        .to_vec();
    assert_eq!(proved(&bytes), (calls, Some(Disposition::Clean), None));
    // Each local refusal is one Started and one refused Terminal, by its
    // call sequence and tool, and nothing of the request.
    let records: Vec<Value> = bytes
        .lines()
        .map(|line| serde_json::from_str(&line.unwrap()).unwrap())
        .collect();
    let admin = [
        json!({"record": "started", "seq": 4, "call": 2, "tool": "admin"}),
        json!({"record": "terminal", "seq": 5, "call": 2, "tool": "admin",
               "outcome": {"refused": "ungranted"}}),
    ];
    assert_eq!(records[3..5], admin);
    let pairs = ["started", "terminal"].map(|kind| {
        let kind = |record: &&Value| record["record"] == kind;
        records.iter().filter(kind).count()
    });
    assert_eq!(pairs, [4, 4]);
}

#[test]
fn at_the_call_bound_the_next_frame_stays_unread_and_closure_says_why() {
    let (mut ledger, disk) = healthy();
    let frames = "resolve\n".repeat(4097);
    let mut input = std::io::Cursor::new(frames.as_bytes());
    let mut accepted = 0;
    // The serving loop asks for room before it reads each frame.
    let limit = loop {
        let room = match ledger.room() {
            Ok(room) => room,
            Err(failure) => break failure,
        };
        let mut frame = String::new();
        input.read_line(&mut frame).unwrap();
        let begun = room.begin(frame.trim_end()).unwrap();
        ledger.end(begun, Outcome::Succeeded).unwrap();
        accepted += 1;
    };
    assert_eq!((limit, accepted), (Failure::Limit, 4096));
    // The 4,097th frame is still unread.
    assert_eq!(input.position(), 4096 * 8);
    assert_eq!(input.fill_buf().unwrap(), b"resolve\n");
    let failed = Disposition::Failed(Failure::Limit);
    assert_eq!(ledger.close(), Ok(failed.clone()));
    let calls = vec![ended("resolve", Outcome::Succeeded); 4096];
    assert_eq!(proved(&disk.borrow().bytes), (calls, Some(failed), None));
}

#[test]
fn a_records_terminal_is_reserved_before_it_is_accepted() {
    // The longest Terminal, `{"refused":"ungranted"}` at the largest
    // sequences, fits the 4 KiB record bound exactly with this tool.
    let terminal = json!({"record": "terminal", "seq": u64::MAX, "call": u64::MAX, "tool": "",
                          "outcome": {"refused": "ungranted"}});
    let fits = 4096 - terminal.to_string().len() - 1;
    let (mut ledger, disk) = healthy();
    let at = "t".repeat(fits);
    let begun = ledger.room().unwrap().begin(&at).unwrap();
    ledger
        .end(begun, Outcome::Refused(Denial::Ungranted))
        .unwrap();
    // One byte more is not accepted, and nothing is written: the ledger's
    // limit, which the closure carries, the call before it kept.
    let before = disk.borrow().appends;
    let over = ledger.room().unwrap().begin(&format!("{at}t"));
    assert_eq!(over.unwrap_err(), Unrecorded::Limit);
    assert_eq!(disk.borrow().appends, before);
    let failed = Disposition::Failed(Failure::Limit);
    assert_eq!(ledger.close(), Ok(failed.clone()));
    let calls = vec![ended(&at, Outcome::Refused(Denial::Ungranted))];
    assert_eq!(proved(&disk.borrow().bytes), (calls, Some(failed), None));
    // An Opened over the bound is not written either.
    let mut long = owner();
    long.site = "s".repeat(4096);
    let disk = Rc::new(RefCell::new(0));
    let counted = Rc::clone(&disk);
    let append = move |_: &[u8]| {
        *counted.borrow_mut() += 1;
        Ok(())
    };
    let refused = Ledger::open_with(append, &long, PLAN).err();
    assert_eq!((refused, *disk.borrow()), (Some(Unrecorded::Record), 0));
}

/// What a ledger whose `nth` append meets `fault` proves once the session
/// has gone as far as it could: one call to `resolve` that succeeded, then
/// closure; and the answers its writer gave.
fn faulted(nth: usize, fault: Fault) -> (Vec<Result<(), Unrecorded>>, Proved) {
    let (ledger, disk) = opened(Some((nth, fault)));
    let mut answers = Vec::new();
    let mut say = |answer| answers.push(answer);
    if let Ok(mut ledger) = ledger.map_err(|refused| say(Err(refused))) {
        let begun = ledger.room().unwrap().begin("resolve");
        let begun = begun.map_err(|refused| say(Err(refused)));
        if let Ok(begun) = begun {
            say(ledger.end(begun, Outcome::Succeeded));
        }
        say(ledger.close().map(drop));
    }
    let bytes = disk.borrow().bytes.clone();
    (answers, proved(&bytes))
}

#[test]
fn a_record_not_made_whole_is_never_acted_on_and_only_its_prefix_verifies() {
    let unrecorded = || Err::<(), _>(Unrecorded::Record);
    let ok = || Ok::<_, Unrecorded>(());
    let resolve = |outcome| vec![("resolve".to_string(), outcome)];
    let succeeded = Some(Outcome::Succeeded);
    for (case, (nth, fault), answers, recovered) in [
        // The Opened: no ledger, so nothing starts.
        (
            "opened",
            (1, Fault::Unwritten),
            vec![unrecorded()],
            (vec![], None, Some(Invalid::Lifecycle)),
        ),
        (
            "opened",
            (1, Fault::Torn),
            vec![unrecorded()],
            (vec![], None, Some(Invalid::Partial)),
        ),
        (
            "opened",
            (1, Fault::Unacknowledged),
            vec![unrecorded()],
            (vec![], None, Some(Invalid::Lifecycle)),
        ),
        // The Started: no call is accepted, so none is forwarded, and the
        // ledger records nothing more, its closure included.
        (
            "started",
            (2, Fault::Unwritten),
            vec![unrecorded(), unrecorded()],
            (vec![], None, Some(Invalid::Lifecycle)),
        ),
        (
            "started",
            (2, Fault::Torn),
            vec![unrecorded(), unrecorded()],
            (vec![], None, Some(Invalid::Partial)),
        ),
        // Its acknowledgement lost, the complete Started still verifies,
        // and the call is interrupted.
        (
            "started",
            (2, Fault::Unacknowledged),
            vec![unrecorded(), unrecorded()],
            (resolve(None), None, Some(Invalid::Lifecycle)),
        ),
        // The Terminal: nothing is delivered, and the call stays begun.
        (
            "terminal",
            (3, Fault::Unwritten),
            vec![unrecorded(), unrecorded()],
            (resolve(None), None, Some(Invalid::Lifecycle)),
        ),
        (
            "terminal",
            (3, Fault::Torn),
            vec![unrecorded(), unrecorded()],
            (resolve(None), None, Some(Invalid::Partial)),
        ),
        (
            "terminal",
            (3, Fault::Unacknowledged),
            vec![unrecorded(), unrecorded()],
            (resolve(succeeded), None, Some(Invalid::Lifecycle)),
        ),
        // The closure: the verified prefix, never a success.
        (
            "closed",
            (4, Fault::Unwritten),
            vec![ok(), unrecorded()],
            (resolve(succeeded), None, Some(Invalid::Lifecycle)),
        ),
        (
            "closed",
            (4, Fault::Torn),
            vec![ok(), unrecorded()],
            (resolve(succeeded), None, Some(Invalid::Partial)),
        ),
    ] {
        let expected = (answers, recovered);
        assert_eq!(
            (case, fault as u8, faulted(nth, fault)),
            (case, fault as u8, expected)
        );
    }
    // A closure synced whole whose acknowledgement was lost is closed; the
    // writer still said it was not recorded.
    let (answers, recovered) = faulted(4, Fault::Unacknowledged);
    assert_eq!(answers, [ok(), unrecorded()]);
    assert_eq!(
        recovered,
        (resolve(succeeded), Some(Disposition::Clean), None)
    );
}

#[test]
fn closure_is_refused_while_a_call_is_unresolved() {
    let (mut ledger, disk) = healthy();
    let begun = ledger.room().unwrap().begin("resolve").unwrap();
    drop(begun);
    let appends = disk.borrow().appends;
    assert_eq!(ledger.close(), Err(Unrecorded::Record));
    assert_eq!(disk.borrow().appends, appends);
    let bytes = disk.borrow().bytes.clone();
    let calls = vec![("resolve".to_string(), None)];
    assert_eq!(proved(&bytes), (calls, None, Some(Invalid::Lifecycle)));
}

#[test]
fn a_call_begun_while_another_is_unresolved_ends_only_refused() {
    // The control: a call is active, a second is refused locally as
    // active, then the first is answered.
    let (mut ledger, disk) = healthy();
    let first = ledger.room().unwrap().begin("resolve").unwrap();
    let second = ledger.room().unwrap().begin("read").unwrap();
    assert_eq!(ledger.end(second, Outcome::Refused(Denial::Active)), Ok(()));
    assert_eq!(ledger.end(first, Outcome::Succeeded), Ok(()));
    assert_eq!(ledger.close(), Ok(Disposition::Clean));
    let bytes = disk.borrow().bytes.clone();
    let both = vec![
        ended("resolve", Outcome::Succeeded),
        ended("read", Outcome::Refused(Denial::Active)),
    ];
    assert_eq!(proved(&bytes), (both, Some(Disposition::Clean), None));
    // The second answered as though forwarded is not recorded, and so the
    // closure waits on it.
    let (mut ledger, disk) = healthy();
    let first = ledger.room().unwrap().begin("resolve").unwrap();
    let second = ledger.room().unwrap().begin("read").unwrap();
    let appends = disk.borrow().appends;
    let answered = ledger.end(second, Outcome::Succeeded);
    assert_eq!(answered, Err(Unrecorded::Record));
    assert_eq!(disk.borrow().appends, appends);
    assert_eq!(ledger.end(first, Outcome::Succeeded), Ok(()));
    assert_eq!(ledger.close(), Err(Unrecorded::Record));
    // A settled ledger that ends the second call so, before or after the
    // first, verifies only up to that Terminal.
    let lines: Vec<String> = bytes.lines().map(Result::unwrap).collect();
    let [o, s1, s2, c] = [0, 1, 2, 5].map(|line| lines[line].as_str());
    let succeeded = |seq: u64, call: u64, tool: &str| {
        let terminal = json!({"record": "terminal", "seq": seq, "call": call, "tool": tool,
                              "outcome": "succeeded"});
        terminal.to_string()
    };
    let (second_first, first_after) = (succeeded(4, 2, "read"), succeeded(5, 1, "resolve"));
    let (first_first, second_after) = (succeeded(4, 1, "resolve"), succeeded(5, 2, "read"));
    let unresolved = |tool: &str| (tool.to_string(), None);
    let cases: [(&str, Vec<&str>, Proved); 2] = [
        (
            "the second first",
            vec![o, s1, s2, &second_first, &first_after, c],
            (
                vec![unresolved("resolve"), unresolved("read")],
                None,
                Some(Invalid::Lifecycle),
            ),
        ),
        (
            "the second after",
            vec![o, s1, s2, &first_first, &second_after, c],
            (
                vec![ended("resolve", Outcome::Succeeded), unresolved("read")],
                None,
                Some(Invalid::Lifecycle),
            ),
        ),
    ];
    for (case, lines, expected) in cases {
        assert_eq!((case, proved(&joined(&lines))), (case, expected));
    }
}

/// The lines of a ledger that opened, accepted `resolve` as call 1 and
/// ended it, and closed Clean, as the writer wrote them.
fn written() -> Vec<String> {
    let (mut ledger, disk) = healthy();
    call(&mut ledger, "resolve", Outcome::Succeeded);
    ledger.close().unwrap();
    let bytes = disk.borrow().bytes.clone();
    bytes.lines().map(Result::unwrap).collect()
}

/// `lines` as a ledger's bytes, each with its newline.
fn joined(lines: &[&str]) -> Vec<u8> {
    lines
        .iter()
        .flat_map(|line| format!("{line}\n").into_bytes())
        .collect()
}

/// A Terminal of call `call` to `tool` at record `seq`, failed.
fn terminal_of(seq: u64, call: u64, tool: &str) -> String {
    let terminal = json!({"record": "terminal", "seq": seq, "call": call, "tool": tool,
                          "outcome": "failed"});
    terminal.to_string()
}

/// A Started of call `call` to `tool` at record `seq`.
fn started_of(seq: u64, call: u64, tool: &str) -> String {
    json!({"record": "started", "seq": seq, "call": call, "tool": tool}).to_string()
}

#[test]
fn a_ledger_out_of_d7s_order_verifies_only_its_prefix() {
    let written = written();
    let [o, s, t, c] = [0, 1, 2, 3].map(|line| written[line].as_str());
    let resolved = || vec![ended("resolve", Outcome::Succeeded)];
    let begun = || vec![("resolve".to_string(), None)];
    let (clean, lifecycle) = (Some(Disposition::Clean), Some(Invalid::Lifecycle));
    let (reopened, closed_early) = (o.replace("\"seq\":1", "\"seq\":2"), c.replace(":4", ":3"));
    let (again, after) = (c.replace(":4", ":5"), started_of(5, 2, "read"));
    let (before, zero) = (terminal_of(2, 1, "resolve"), terminal_of(2, 0, "resolve"));
    let (twice, other) = (terminal_of(4, 1, "resolve"), terminal_of(3, 1, "read"));
    let unopened = started_of(1, 1, "resolve");
    let cases: [(&str, Vec<&str>, Proved); 11] = [
        (
            "as written",
            vec![o, s, t, c],
            (resolved(), clean.clone(), None),
        ),
        ("no Opened", vec![&unopened], (vec![], None, lifecycle)),
        (
            "a second Opened",
            vec![o, &reopened, &closed_early],
            (vec![], None, lifecycle),
        ),
        (
            "Terminal before Started",
            vec![o, &before],
            (vec![], None, lifecycle),
        ),
        (
            "Terminal of call zero",
            vec![o, &zero],
            (vec![], None, lifecycle),
        ),
        (
            "a second Terminal",
            vec![o, s, t, &twice],
            (resolved(), None, Some(Invalid::Conflict)),
        ),
        (
            "another tool's Terminal",
            vec![o, s, &other],
            (begun(), None, Some(Invalid::Conflict)),
        ),
        (
            "Closed before a Terminal",
            vec![o, s, &closed_early],
            (begun(), None, lifecycle),
        ),
        (
            "a record after Closed",
            vec![o, s, t, c, &after],
            (resolved(), clean.clone(), lifecycle),
        ),
        (
            "a second Closed",
            vec![o, s, t, c, &again],
            (resolved(), clean, lifecycle),
        ),
        ("no Closed", vec![o, s, t], (resolved(), None, lifecycle)),
    ];
    for (case, lines, expected) in cases {
        assert_eq!((case, proved(&joined(&lines))), (case, expected));
    }
}

#[test]
fn a_ledger_out_of_sequence_of_another_owner_or_malformed_verifies_only_its_prefix() {
    let written = written();
    let [o, s, t, c] = [0, 1, 2, 3].map(|line| written[line].as_str());
    let (gap, call_gap) = (s.replace("\"seq\":2", "\"seq\":3"), s.replace(":1,", ":2,"));
    let (owner, plan) = (
        o.replace("research#0", "research#1"),
        o.replace(PLAN, &"5".repeat(64)),
    );
    let unknown = s.replace("\"tool\"", "\"args\":{},\"tool\"");
    let positional = json!(["started", 2, 1, "resolve"]).to_string();
    let fields = json!([
        "a".repeat(64),
        "run-1",
        super::EFFECT,
        super::ATTEMPT,
        "research",
        "x"
    ]);
    let owned = json!({"record": "opened", "seq": 1, "plan": PLAN, "owner": fields}).to_string();
    let (trailing, long) = (format!("{s} {{}}"), s.replace("resolve", &"r".repeat(4096)));
    let malformed = || (vec![], None, Some(Invalid::Malformed));
    let cases: [(&str, Vec<&str>, Proved); 10] = [
        (
            "a record sequence gap",
            vec![o, &gap],
            (vec![], None, Some(Invalid::Gap)),
        ),
        (
            "a call sequence gap",
            vec![o, &call_gap],
            (vec![], None, Some(Invalid::Gap)),
        ),
        (
            "another owner",
            vec![&owner],
            (vec![], None, Some(Invalid::Owner)),
        ),
        (
            "another plan",
            vec![&plan],
            (vec![], None, Some(Invalid::Owner)),
        ),
        ("not JSON", vec![o, "{\"record\":", c], malformed()),
        ("an unknown field", vec![o, &unknown], malformed()),
        ("a positional record", vec![o, &positional], malformed()),
        ("a positional owner", vec![&owned], malformed()),
        ("bytes after the record", vec![o, &trailing], malformed()),
        ("a record over 4 KiB", vec![o, &long], malformed()),
    ];
    for (case, lines, expected) in cases {
        assert_eq!((case, proved(&joined(&lines))), (case, expected));
    }
    // A record with no newline is partial, however whole its object.
    let mut bytes = joined(&[o, s, t]);
    bytes.extend_from_slice(c.as_bytes());
    let resolved = vec![ended("resolve", Outcome::Succeeded)];
    assert_eq!(proved(&bytes), (resolved, None, Some(Invalid::Partial)));
}

#[test]
fn a_ledger_of_more_than_4096_calls_is_out_of_order() {
    let (mut ledger, disk) = healthy();
    for _ in 0..4096 {
        call(&mut ledger, "resolve", Outcome::Succeeded);
    }
    let mut bytes = disk.borrow().bytes.clone();
    let next = json!({"record": "started", "seq": 8194, "call": 4097, "tool": "resolve"});
    bytes.extend_from_slice(format!("{next}\n").as_bytes());
    let calls = vec![ended("resolve", Outcome::Succeeded); 4096];
    assert_eq!(proved(&bytes), (calls, None, Some(Invalid::Lifecycle)));
}

#[test]
fn a_plans_ledger_is_created_once_beside_it_and_this_users_alone() {
    let root = Root::new();
    let dir = std::fs::File::open(&root.path).unwrap();
    let plan = std::ffi::OsStr::new("cap-library-docs.json");
    let ledger = Ledger::open(&dir, plan, &owner(), PLAN).unwrap();
    assert_eq!(ledger.close(), Ok(Disposition::Clean));
    let path = of(&root.path.join(plan));
    let held = std::fs::symlink_metadata(&path).unwrap();
    assert_eq!(
        (held.is_file(), held.permissions().mode() & 0o777),
        (true, 0o600)
    );
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(proved(&bytes), (vec![], Some(Disposition::Clean), None));
    // A second lifetime of the plan is refused, its ledger untouched.
    let again = Ledger::open(&dir, plan, &owner(), PLAN).err();
    assert_eq!(again, Some(Unrecorded::Reopened));
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    // So is one whose ledger's name is a link, which is not followed.
    let target = root.path.join("elsewhere");
    std::os::unix::fs::symlink(&target, of(&root.path.join("linked.json"))).unwrap();
    let linked = Ledger::open(&dir, "linked.json".as_ref(), &owner(), PLAN).err();
    assert_eq!(
        (linked, target.exists()),
        (Some(Unrecorded::Reopened), false)
    );
    // A ledger that cannot be created is not recorded.
    let missing = Ledger::open(&dir, "missing/plan.json".as_ref(), &owner(), PLAN).err();
    assert_eq!(missing, Some(Unrecorded::Record));
}

#[cfg(target_os = "linux")]
#[test]
fn a_ledger_whose_directory_cannot_be_synced_records_nothing() {
    use rustix::fs::{Mode, OFlags};
    let root = Root::new();
    // A path-only handle names the directory to create in, but the kernel
    // refuses to sync it.
    let flags = OFlags::PATH | OFlags::DIRECTORY | OFlags::CLOEXEC;
    let dir = rustix::fs::open(&root.path, flags, Mode::empty()).unwrap();
    let plan = std::ffi::OsStr::new("cap-library-docs.json");
    let refused = Ledger::open(&dir, plan, &owner(), PLAN).err();
    // The ledger is created, and holds no Opened: the directory is synced
    // before any record, and nothing is recorded when it is not.
    let kept = std::fs::read_to_string(of(&root.path.join(plan))).unwrap();
    assert_eq!((refused, kept.as_str()), (Some(Unrecorded::Record), ""));
    let unclosed = (vec![], None, Some(Invalid::Lifecycle));
    assert_eq!(proved(kept.as_bytes()), unclosed);
    // It is still the plan's one lifetime.
    let synced = std::fs::File::open(&root.path).unwrap();
    let again = Ledger::open(&synced, plan, &owner(), PLAN).err();
    assert_eq!(again, Some(Unrecorded::Reopened));
}

/// What the ledger of `sealed`'s plan, sealed as `plan` at `digest`,
/// proves.
fn served(sealed: &Sealed, plan: &Value, digest: &str) -> Proved {
    let owner: Owner = serde_json::from_value(plan["owner"].clone()).unwrap();
    let bytes = std::fs::read(of(&sealed.locator)).unwrap();
    let (lifecycle, invalid) = recover(&bytes, &owner, digest);
    flat(&lifecycle, invalid)
}

/// What the ledger of `sealed`'s plan proves, for the owner and digest of
/// the plan as it is sealed.
pub(super) fn sealed_ledger(sealed: &Sealed) -> Proved {
    let bytes = std::fs::read(&sealed.locator).unwrap();
    let plan: Value = serde_json::from_slice(&bytes).unwrap();
    let digest = brokkr_core::canonical::sha256_bytes(&bytes);
    served(sealed, &plan, &digest)
}

/// The answer to `plan` sealed, and what its ledger proves.
fn answered(sealed: &Sealed, plan: &Value) -> ((Option<i32>, String), Proved) {
    let digest = sealed.seal_bytes(plan.to_string().as_bytes());
    let answer = sealed.serve(&digest);
    (answer, served(sealed, plan, &digest))
}

/// What a ledger closed Failed with `refusal` and no call proves.
pub(super) fn failed_with(refusal: Refusal) -> Proved {
    let failure = Failure::Refused(refusal);
    (vec![], Some(Disposition::Failed(failure)), None)
}

#[test]
fn an_unbound_plan_opens_no_ledger() {
    if !bindable() {
        return;
    }
    let sealed = Sealed::new();
    let plan = sealed.plan();
    // A digest the inventory does not pin, and a plan bound by its digest
    // but not of the closed shape the engine seals.
    sealed.seal_bytes(plan.to_string().as_bytes());
    assert_eq!(sealed.serve(super::DIGEST), refused(Refusal::Unbound));
    let mut misnamed = plan.clone();
    set(&mut misnamed, "/server", json!("cap-other"));
    let misnamed = sealed.seal_bytes(misnamed.to_string().as_bytes());
    assert_eq!(sealed.serve(&misnamed), refused(Refusal::Unbound));
    assert!(!of(&sealed.locator).exists());
    // The control: the plan sealed as it is binds, and opens its ledger.
    let digest = sealed.seal_bytes(plan.to_string().as_bytes());
    binds("bound", sealed.serve(&digest));
    assert!(of(&sealed.locator).exists());
}

#[test]
fn a_bound_plan_refused_closes_failed_with_its_cause_and_no_call() {
    if !bindable() {
        return;
    }
    let sealed = Sealed::new();
    let plan = sealed.plan();
    // Refused before any box: a binding name, then a fixed key.
    let mut named = plan.clone();
    set(&mut named, "/secrets", json!(["bad name"]));
    let ((code, stderr), proved) = answered(&sealed, &named);
    let cause = stderr
        .strip_prefix("error: ")
        .unwrap()
        .trim_end()
        .to_string();
    assert_eq!(
        (code, &proved),
        (Some(1), &failed_with(Refusal::Name(cause)))
    );
    let mut fixed = plan.clone();
    set(&mut fixed, "/secrets", json!(["HOME"]));
    let refusal = Refusal::StartupInputs;
    let expected = (refused(refusal.clone()), failed_with(refusal));
    assert_eq!(answered(&sealed, &fixed), expected);
    // Past the box's preparation: its own cause, admitted where a box
    // stands, with serving's cause.
    let (answer, proved) = answered(&sealed, &plan);
    let cause = match unservable() {
        None => Refusal::ServingIncomplete,
        Some(_) => Refusal::Identity,
    };
    let cause = match cfg!(target_os = "linux") {
        true => cause,
        false => Refusal::Unavailable,
    };
    assert_eq!(answer, past_prepare(cause.clone()));
    assert_eq!(proved, failed_with(cause));
    // The broker wrote its ledger and no journal, where it ran or under
    // the engine's HOME.
    let journals = [sealed.path(".forge"), sealed.home.join(".forge")];
    assert_eq!(journals.map(|journal| journal.exists()), [false, false]);
}

#[test]
fn a_box_that_cannot_stand_closes_failed_with_no_call() {
    if let Some(reason) = unservable() {
        return skip(&reason);
    }
    if !bindable() {
        return;
    }
    let sealed = Sealed::new();
    let plan = sealed.plan();
    let digest = sealed.seal_bytes(plan.to_string().as_bytes());
    assert_eq!(sealed.serve_unboxed(&digest), refused(Refusal::Unavailable));
    let proved = served(&sealed, &plan, &digest);
    assert_eq!(proved, failed_with(Refusal::Unavailable));
}

#[test]
fn a_second_lifetime_of_a_plan_is_refused_before_anything_starts() {
    if !bindable() {
        return;
    }
    let sealed = Sealed::new();
    let plan = sealed.plan();
    let digest = sealed.seal_bytes(plan.to_string().as_bytes());
    binds("first", sealed.serve(&digest));
    let first = std::fs::read(of(&sealed.locator)).unwrap();
    // Where a box stands, the first lifetime looked its binding up; the
    // second looks nothing up.
    #[cfg(target_os = "linux")]
    let reads = super::readiness::Reads::of(&sealed);
    let home = sealed.home.clone();
    let again = sealed.serve_again(&sealed.locator, &digest, |command| {
        command.env("HOME", &home);
    });
    let reopened = (Some(1), format!("error: {}\n", Unrecorded::Reopened));
    assert_eq!(again, reopened);
    #[cfg(target_os = "linux")]
    assert_eq!(reads.lookups(), 0);
    assert_eq!(std::fs::read(of(&sealed.locator)).unwrap(), first);
    assert!(!sealed.path("started").exists());
}
