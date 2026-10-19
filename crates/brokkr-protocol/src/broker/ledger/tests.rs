//! A ledger's records reach its file through the one persistence the broker
//! uses (CR3; D7): each line written whole, then its data synced, before the
//! writer answers. Each case puts a medium that refuses in place of the real
//! file at one record, so the failure is the kernel's own: a full device
//! refuses the write, and a pipe takes the write but refuses the sync.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::mem::ManuallyDrop;
use std::os::fd::{AsFd, AsRawFd, FromRawFd, OwnedFd, RawFd};

use super::retention::{share, BUDGET, RESERVATION};
use super::{recover, Denial, Disposition, Entry, Failure, Invalid, Ledger, Lifecycle, Outcome};
use super::{Unrecorded, RECORD_MAX};
use crate::broker::Owner;

/// The plan digest every case opens its ledger for.
const PLAN: &str = "5ea1ed0000000000000000000000000000000000000000000000000000000000";

/// The owner every case opens its ledger for.
fn owner() -> Owner {
    Owner {
        repo: "a".repeat(64),
        run: "run-1".into(),
        effect: "effect-1".into(),
        attempt: "attempt-1".into(),
        site: "research".into(),
        instance: "research#0".into(),
    }
}

/// What refuses one record in place of the ledger's file.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Medium {
    /// A full device, which refuses the write.
    Full,
    /// A pipe, which takes the write but cannot be synced.
    Pipe,
}

/// From now on, the ledger file's descriptor `held` names `medium`, closed
/// by an exec, so no process another test starts meanwhile inherits it.
fn swap(held: RawFd, medium: impl AsFd) {
    use rustix::io::{dup3, fcntl_getfd, DupFlags, FdFlags};
    // SAFETY: `held` is the descriptor of the file the case or its open
    // ledger owns, open across this call; `ManuallyDrop` leaves closing it
    // to its owner.
    let mut held = ManuallyDrop::new(unsafe { OwnedFd::from_raw_fd(held) });
    dup3(medium, &mut held, DupFlags::CLOEXEC).unwrap();
    assert_eq!(fcntl_getfd(&*held), Ok(FdFlags::CLOEXEC));
}

/// What one session, one call to `resolve` that succeeded and then closure,
/// made of a ledger whose `nth` record, counted from one, met `medium` in
/// place of its file: the writer's answers, the bytes the file kept, and
/// the bytes the medium took.
fn session(nth: usize, medium: Medium) -> (Vec<Result<(), Unrecorded>>, String, String) {
    let flags = rustix::fs::MemfdFlags::CLOEXEC;
    let file = File::from(rustix::fs::memfd_create("ledger", flags).unwrap());
    let mut kept = file.try_clone().unwrap();
    let held = file.as_raw_fd();
    let (taken, refusing): (_, OwnedFd) = match medium {
        Medium::Full => (
            None,
            File::options()
                .write(true)
                .open("/dev/full")
                .unwrap()
                .into(),
        ),
        Medium::Pipe => {
            let (reader, writer) = std::io::pipe().unwrap();
            (Some(reader), writer.into())
        }
    };
    let at = |record| {
        if record == nth {
            swap(held, &refusing);
        }
    };
    let mut answers = Vec::new();
    at(1);
    match Ledger::durable(file, (&owner(), PLAN), &[]) {
        Err(refused) => answers.push(Err(refused)),
        Ok(mut ledger) => {
            at(2);
            match ledger.room().unwrap().begin("resolve") {
                Ok(begun) => {
                    at(3);
                    answers.push(ledger.end(begun, Outcome::Succeeded));
                }
                Err(refused) => answers.push(Err(refused)),
            }
            at(4);
            answers.push(ledger.close().map(drop));
        }
    }
    drop(refusing);
    let mut bytes = String::new();
    kept.seek(SeekFrom::Start(0)).unwrap();
    kept.read_to_string(&mut bytes).unwrap();
    let mut took = String::new();
    if let Some(mut taken) = taken {
        taken.read_to_string(&mut took).unwrap();
    }
    (answers, bytes, took)
}

#[test]
fn each_record_is_written_whole_then_synced_before_the_writer_answers() {
    let (ok, unrecorded) = (Ok(()), Err(Unrecorded::Record));
    // The control: Opened, Started, Terminal and Closed, all kept.
    let (answers, written, took) = session(0, Medium::Pipe);
    assert_eq!((answers, took), (vec![ok, ok], String::new()));
    let (lifecycle, invalid) = recover(written.as_bytes(), &owner(), PLAN);
    let resolve = |outcome| vec![("resolve".to_string(), outcome)];
    let succeeded = resolve(Some(Outcome::Succeeded));
    assert_eq!(
        (lifecycle.calls(), lifecycle.closed(), invalid),
        (&succeeded[..], Some(&Disposition::Clean), None)
    );
    let lines: Vec<&str> = written.split_inclusive('\n').collect();
    assert_eq!(lines.len(), 4);
    for (nth, answers, calls) in [
        (1, vec![unrecorded], vec![]),
        (2, vec![unrecorded, unrecorded], vec![]),
        (3, vec![unrecorded, unrecorded], resolve(None)),
        (4, vec![ok, unrecorded], succeeded.clone()),
    ] {
        for medium in [Medium::Full, Medium::Pipe] {
            // The file keeps the records before this one. The pipe took this
            // record whole before its sync was refused, and nothing after
            // it; the full device took nothing.
            let taken = match medium {
                Medium::Full => String::new(),
                Medium::Pipe => lines[nth - 1].to_string(),
            };
            let expected = (answers.clone(), lines[..nth - 1].concat(), taken);
            let (said, kept, took) = session(nth, medium);
            assert_eq!(
                (nth, medium, (said, kept.clone(), took)),
                (nth, medium, expected)
            );
            // Only the verified prefix recovers, never a closure.
            let (lifecycle, invalid) = recover(kept.as_bytes(), &owner(), PLAN);
            let recovered = (lifecycle.calls(), lifecycle.closed(), invalid);
            let unclosed = (&calls[..], None, Some(Invalid::Lifecycle));
            assert_eq!((nth, medium, recovered), (nth, medium, unclosed));
        }
    }
}

/// `value` bound as `STATE`, from a store on a canonical temporary root.
fn state(value: &str) -> Vec<crate::secret::BoundSecret> {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().canonicalize().unwrap().join("secrets.env");
    crate::secret::store_set(&store, "STATE", value).unwrap();
    crate::secret::resolve_bindings(&store, &["STATE".to_string()]).unwrap()
}

/// A ledger opened for the cases' plan with no binding, and every byte it
/// appends.
fn appending<'b>() -> (Ledger<'b>, std::rc::Rc<std::cell::RefCell<Vec<u8>>>) {
    let appended = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let sink = std::rc::Rc::clone(&appended);
    let append = move |line: &[u8]| {
        sink.borrow_mut().extend_from_slice(line);
        Ok(())
    };
    let ledger = Ledger::open_with(append, (&owner(), PLAN), &[]).unwrap();
    (ledger, appended)
}

#[test]
fn binding_rereads_the_very_opened_the_ledger_appended() {
    // Decision 0071 ruling 5: the Opened is derived once, so the line
    // appended at open is the line binding re-reads, byte for byte. A
    // binding that is that whole line, newline aside, is found in it, and
    // refused; the Opened stays as it was written.
    let owner = concat!(
        r#"{"repo":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","#,
        r#""run":"run-1","effect":"effect-1","attempt":"attempt-1","#,
        r#""site":"research","instance":"research#0"}"#,
    );
    let opened = format!(r#"{{"record":"opened","seq":1,"owner":{owner},"plan":"{PLAN}"}}"#);
    let (whole, unrelated) = (state(&opened), state("unrelated"));
    let (mut ledger, appended) = appending();
    assert_eq!(*appended.borrow(), format!("{opened}\n").into_bytes());
    assert_eq!(ledger.bind(&whole), Err(Unrecorded::Masking));
    assert_eq!(*appended.borrow(), format!("{opened}\n").into_bytes());
    // The control: an unrelated binding guards the Opened it re-reads.
    let (mut ledger, _) = appending();
    assert_eq!(ledger.bind(&unrelated), Ok(()));
}

#[test]
fn a_ledger_is_bound_once_and_a_second_binding_never_weakens_its_guard() {
    // STATE=0002 is the Closed's sequence, 2, by what it is worth. A second
    // binding, of no value or of fewer, is refused as a record the ledger
    // cannot make, and the first guard stays: the Closed is refused and
    // never appended (operator ruling 2026-10-13).
    let (bound, unrelated) = (state("0002"), state("unrelated"));
    let fewer: &[crate::secret::BoundSecret] = &[];
    for second in [fewer, &unrelated] {
        let (mut ledger, appended) = appending();
        assert_eq!(ledger.bind(&bound), Ok(()));
        assert_eq!(ledger.bind(second), Err(Unrecorded::Record));
        let opened = appended.borrow().clone();
        assert_eq!(ledger.close(), Err(Unrecorded::Masking));
        assert_eq!(*appended.borrow(), opened);
    }
    // A ledger opened with its bindings is bound by them already.
    let append = |_: &[u8]| Ok(());
    let mut ledger = Ledger::open_with(append, (&owner(), PLAN), &bound).unwrap();
    assert_eq!(ledger.bind(fewer), Err(Unrecorded::Record));
    assert_eq!(ledger.close(), Err(Unrecorded::Masking));
    // A second binding refused latches the masking cause: a guard the Closed
    // passes closes it Failed, never Clean, however the session ends.
    let (mut ledger, _) = appending();
    assert_eq!(ledger.bind(&unrelated), Ok(()));
    assert_eq!(ledger.bind(fewer), Err(Unrecorded::Record));
    assert_eq!(ledger.close(), Ok(Disposition::Failed(Failure::Masking)));
    // The control: bound once by an unrelated value, the Closed is written.
    let (mut ledger, _) = appending();
    assert_eq!(ledger.bind(&unrelated), Ok(()));
    assert_eq!(ledger.close(), Ok(Disposition::Clean));
}

/// The sha256 of `bytes`, as the broker cites it.
fn sha256(bytes: &[u8]) -> String {
    use sha2::Digest;
    hex::encode(sha2::Sha256::digest(bytes))
}

/// Each line of `bytes`, as JSON.
fn records(bytes: &[u8]) -> Vec<serde_json::Value> {
    let lines = bytes.split_inclusive(|byte| *byte == b'\n');
    lines
        .map(|line| serde_json::from_slice(line).unwrap())
        .collect()
}

#[test]
fn a_retaining_ledger_stages_each_answer_beside_it_before_the_terminal_citing_it() {
    // U8b: the prepared bytes are staged in the attempt directory as
    // `<plan>.<call>.response`, this user's alone, before the Terminal that
    // cites their digest and length. A stage whose name is taken already
    // is not made, its Terminal not written, and the name's bytes kept.
    let root = tempfile::tempdir().unwrap();
    let dir = File::open(root.path()).unwrap();
    let plan = std::ffi::OsStr::new("cap-docs.json");
    let mut ledger = Ledger::open(&dir, plan, (&owner(), PLAN), &[]).unwrap();
    ledger.retain(OwnedFd::from(dir.try_clone().unwrap()), plan, 1);
    let answer = br#"{"result":{"content":[]}}"#;
    let mut call = ledger.room().unwrap().begin("resolve").unwrap();
    assert_eq!(ledger.reserve(&mut call), Ok(()));
    let answered = ledger.answered(call, Outcome::Succeeded, (answer, &sha256(answer)));
    assert_eq!(answered, Ok(()));
    let staged = root.path().join("cap-docs.json.1.response");
    assert_eq!(std::fs::read(&staged).unwrap(), answer);
    use std::os::unix::fs::PermissionsExt;
    let mode = std::fs::metadata(&staged).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o600);
    let taken = root.path().join("cap-docs.json.2.response");
    std::fs::write(&taken, b"earlier").unwrap();
    let mut call = ledger.room().unwrap().begin("resolve").unwrap();
    assert_eq!(ledger.reserve(&mut call), Ok(()));
    let answered = ledger.answered(call, Outcome::Succeeded, (answer, &sha256(answer)));
    assert_eq!(answered, Err(Unrecorded::Record));
    assert_eq!(std::fs::read(&taken).unwrap(), b"earlier");
    let written = std::fs::read(root.path().join("cap-docs.json.ledger")).unwrap();
    let cited = serde_json::json!({"sha256": sha256(answer), "bytes": answer.len()});
    assert_eq!(records(&written)[2]["response"], cited);
    let (lifecycle, invalid) = recover(&written, &owner(), PLAN);
    let calls = [
        ("resolve".to_string(), Some(Outcome::Succeeded)),
        ("resolve".to_string(), None),
    ];
    let recovered = (lifecycle.calls(), invalid);
    assert_eq!(recovered, (&calls[..], Some(Invalid::Lifecycle)));
}

/// A ledger in memory retaining as one of `retaining` plans, and how many
/// stages it was asked for.
fn retaining<'b>(retaining: usize) -> (Ledger<'b>, std::rc::Rc<std::cell::Cell<usize>>) {
    let (mut ledger, _) = appending();
    let stages = std::rc::Rc::new(std::cell::Cell::new(0));
    let counted = std::rc::Rc::clone(&stages);
    let stage = move |_, _: &[u8]| {
        counted.set(counted.get() + 1);
        Ok(())
    };
    ledger.retain_with(stage, retaining);
    (ledger, stages)
}

#[test]
fn a_response_its_call_holds_no_room_for_is_never_staged() {
    // Unreserved, or past its reservation: nothing staged, and the call
    // not recorded. The control: at its reservation exactly, staged.
    let (mut ledger, stages) = retaining(1);
    let call = ledger.room().unwrap().begin("resolve").unwrap();
    let answered = ledger.answered(call, Outcome::Succeeded, (b"{}", &sha256(b"{}")));
    assert_eq!((answered, stages.get()), (Err(Unrecorded::Record), 0));
    let over = vec![b'x'; RESERVATION as usize + 1];
    let at = &over[1..];
    let cases = [(&over[..], Err(Unrecorded::Record), 0), (at, Ok(()), 1)];
    for (bytes, recorded, staged) in cases {
        let (mut ledger, stages) = retaining(1);
        let mut call = ledger.room().unwrap().begin("resolve").unwrap();
        ledger.reserve(&mut call).unwrap();
        let answered = ledger.answered(call, Outcome::Succeeded, (bytes, &sha256(bytes)));
        let found = (bytes.len(), answered, stages.get());
        assert_eq!(found, (bytes.len(), recorded, staged));
    }
}

#[test]
fn the_budget_is_divided_by_the_retaining_plans_and_each_call_reserves_8_mib() {
    // Operator ruling 2026-10-15: 256 MiB over the retaining plans the
    // inventory pins; 32 shares fill it exactly, 33 fall under one
    // reservation, and none divides nothing.
    let shares = (share(32), share(33) < RESERVATION, share(0));
    assert_eq!(shares, (8 << 20, true, 0));
    assert_eq!(32 * share(32), BUDGET);
    // One plan's 256 MiB holds 32 reservations, and no 33rd; a call that
    // ends unstaged, here refused as the lifecycle lets a call begun beside
    // others end, returns its own.
    let (mut ledger, _) = retaining(1);
    let mut held = Vec::new();
    for _ in 0..32 {
        let mut call = ledger.room().unwrap().begin("resolve").unwrap();
        assert_eq!(ledger.reserve(&mut call), Ok(()));
        held.push(call);
    }
    let mut last = ledger.room().unwrap().begin("resolve").unwrap();
    assert_eq!(ledger.reserve(&mut last), Err(Denial::Budget));
    let refused = Outcome::Refused(Denial::Active);
    assert_eq!(ledger.end(held.pop().unwrap(), refused), Ok(()));
    assert_eq!(ledger.reserve(&mut last), Ok(()));
    // A plan that retains nothing reserves nothing, and cites no digest.
    let (mut plain, appended) = appending();
    let mut call = plain.room().unwrap().begin("resolve").unwrap();
    assert_eq!(plain.reserve(&mut call), Ok(()));
    let answered = plain.answered(call, Outcome::Succeeded, (b"{}", &sha256(b"{}")));
    assert_eq!(answered, Ok(()));
    assert_eq!(records(&appended.borrow())[2].get("response"), None);
}

#[test]
fn a_terminal_cites_a_sound_response_for_an_answered_call_alone() {
    // CC2: a refusal forwarded nothing, so cites nothing; a digest that is
    // no lowercase sha256, a length past one reservation, a null, or a
    // response read by position, is malformed. The control verifies its
    // call, and is refused only for its missing closure.
    let opened = Lifecycle::new(&owner(), PLAN).opened().line().unwrap();
    let started = br#"{"record":"started","seq":2,"call":1,"tool":"resolve"}"#;
    let ledger = |outcome: &str, response: String| {
        let terminal = format!(
            r#"{{"record":"terminal","seq":3,"call":1,"tool":"resolve","outcome":{outcome},"response":{response}}}"#
        );
        [&opened[..], started, b"\n", terminal.as_bytes(), b"\n"].concat()
    };
    let digest = sha256(b"{}");
    let cited = |digest: &str, bytes: u64| format!(r#"{{"sha256":"{digest}","bytes":{bytes}}}"#);
    let (refused, failed) = (r#"{"refused":"budget"}"#, r#""failed""#);
    let cases = [
        (
            "refused",
            ledger(refused, cited(&digest, 2)),
            Invalid::Lifecycle,
            0,
        ),
        (
            "upper",
            ledger(failed, cited(&digest.to_uppercase(), 2)),
            Invalid::Malformed,
            0,
        ),
        (
            "short",
            ledger(failed, cited(&digest[1..], 2)),
            Invalid::Malformed,
            0,
        ),
        (
            "long",
            ledger(failed, cited(&digest, RESERVATION + 1)),
            Invalid::Malformed,
            0,
        ),
        ("null", ledger(failed, "null".into()), Invalid::Malformed, 0),
        (
            "positional",
            ledger(failed, format!(r#"["{digest}",2]"#)),
            Invalid::Malformed,
            0,
        ),
        (
            "sound",
            ledger(failed, cited(&digest, RESERVATION)),
            Invalid::Lifecycle,
            1,
        ),
    ];
    for (case, bytes, invalid, verified) in cases {
        let (lifecycle, found) = recover(&bytes, &owner(), PLAN);
        let ended = lifecycle
            .calls()
            .iter()
            .filter(|(_, outcome)| outcome.is_some());
        assert_eq!(
            (case, found, ended.count()),
            (case, Some(invalid), verified)
        );
    }
}

#[test]
fn a_retaining_call_is_accepted_only_where_its_longest_terminal_fits() {
    // A tool whose Terminal fits the record bound exactly with the longest
    // outcome is accepted by a plan that retains nothing, and not by one
    // that retains, whose Terminal may cite the longest response.
    let plain = Entry::Terminal {
        seq: u64::MAX,
        call: u64::MAX,
        tool: String::new(),
        outcome: Outcome::LONGEST,
        response: None,
    };
    let tool = "a".repeat(RECORD_MAX - plain.line().unwrap().len());
    let (mut ledger, _) = appending();
    assert_eq!(ledger.room().unwrap().begin(&tool).map(drop), Ok(()));
    let (mut ledger, stages) = retaining(1);
    let refused = ledger.room().unwrap().begin(&tool).map(drop);
    assert_eq!((refused, stages.get()), (Err(Unrecorded::Limit), 0));
    assert_eq!(ledger.close(), Ok(Disposition::Failed(Failure::Limit)));
}
