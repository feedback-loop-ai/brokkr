//! A ledger's records reach its file through the one persistence the broker
//! uses (CR3; D7): each line written whole, then its data synced, before the
//! writer answers. Each case puts a medium that refuses in place of the real
//! file at one record, so the failure is the kernel's own: a full device
//! refuses the write, and a pipe takes the write but refuses the sync.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::mem::ManuallyDrop;
use std::os::fd::{AsFd, AsRawFd, FromRawFd, OwnedFd, RawFd};

use super::{recover, Disposition, Invalid, Ledger, Outcome, Unrecorded};
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

/// From now on, the ledger file's descriptor `held` names `medium`.
fn swap(held: RawFd, medium: impl AsFd) {
    // SAFETY: `held` is the descriptor of the file the case or its open
    // ledger owns, open across this call; `ManuallyDrop` leaves closing it
    // to its owner.
    let mut held = ManuallyDrop::new(unsafe { OwnedFd::from_raw_fd(held) });
    rustix::io::dup2(medium, &mut held).unwrap();
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
    match Ledger::durable(file, &owner(), PLAN) {
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
