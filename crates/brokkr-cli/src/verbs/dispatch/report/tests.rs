//! The lines a pass and the holder's readout print (the controller's R3):
//! one per fact, each through the safe renderer, a spawn failure's line
//! carrying the command that resumes its run, and an ending's words last.

use std::path::Path;

use brokkr_runtime::admission::{AdmissionError, Reason};
use brokkr_runtime::dispatch::{ClaimRefused, Ending, SweepRefused};
use brokkr_runtime::sweep::Swept;
use brokkr_store::{EntryId, StartRefusal, StoreError};
use clap::Parser;

use super::super::tests::{claimed_by, one_claim, OneClaim};
use super::*;
use crate::{Cli, Cmd};

/// A store error that is a contention, as the store words it.
fn contention() -> StoreError {
    StoreError::Contended {
        operation: "queue_link",
        waited_ms: 5,
    }
}

/// A report naming one of every fact, about `claim`.
fn every_fact(claim: Claim) -> Passed {
    let spawned = |pid| Ok(Spawned { pid });
    Passed {
        reconciled: vec![
            Settled::Reclaimed(claim),
            Settled::ToSpawn(claim, "run-r".into()),
        ],
        spawned: vec![Spawning {
            claim,
            run: "run-r".into(),
            spawned: spawned(41),
        }],
        sweep: Some(SweepOutcome::Swept(Swept::default())),
        bound: Some(BoundOutcome::Unreadable(contention())),
        admission_refused: Some(AdmissionError::Unmeasured(EntryId(4))),
        unstarted: vec![Reason::OperatorHold],
        started: vec![
            Started {
                entry: claim.parts().0,
                claim,
                run: "run-s".into(),
                spawned: spawned(42),
            },
            Started {
                entry: claim.parts().0,
                claim,
                run: "run-f".into(),
                spawned: Err(SpawnFailed::Refused("no descriptor".into())),
            },
        ],
        left: vec![Left {
            claim,
            run: "run-l".into(),
        }],
        reclaimed: vec![
            Reclaimed {
                claim,
                refused: Some(StartRefusal::Compile),
                count: 2,
                reason: "compile: it does not parse".into(),
            },
            Reclaimed {
                claim,
                refused: None,
                count: 0,
                reason: "start contended".into(),
            },
        ],
        held: vec![Held {
            claim,
            refusal: StartRefusal::Crossing,
            count: 3,
            reason: "crossing: \u{1b}[2Jmoved".into(),
        }],
        claims_refused: vec![ClaimRefused {
            entry: EntryId(5),
            reason: Reason::OperatorHold,
        }],
        contended: vec![
            Contended {
                subject: Subject::Pass,
                stage: Stage::Judge,
                error: contention(),
            },
            Contended {
                subject: Subject::Entry(EntryId(6)),
                stage: Stage::Claim,
                error: contention(),
            },
            Contended {
                subject: Subject::Claim(claim.parts().0, claim),
                stage: Stage::Spawn,
                error: contention(),
            },
        ],
        waiting: 7,
        held_entries: 8,
    }
}

#[test]
fn a_pass_prints_one_safe_line_per_fact_and_its_counts_last() {
    let OneClaim { db, claim, .. } = one_claim("{}");
    let (entry, seq) = claim.parts();
    let under = format!("claim {seq} of entry {entry}");
    let busy = contention().to_string();
    let unmeasured = AdmissionError::Unmeasured(EntryId(4)).to_string();
    let hold = Reason::OperatorHold.to_string();
    let escaped = safe("crossing: \u{1b}[2Jmoved");
    assert_ne!(escaped, "crossing: \u{1b}[2Jmoved", "the escape is painted");
    let expected = [
        format!("reconciled: {under}, which no run carries, returned to its place"),
        format!("started: run run-r under {under}, driver pid 41"),
        "sweep: freed 0 bytes".to_string(),
        format!("bound unreadable: {busy}; nothing was claimed"),
        format!("admission refused: {unmeasured}; nothing more was claimed"),
        format!("unstarted: {hold}"),
        format!("started: run run-s under {under}, driver pid 42"),
        format!(
            "spawn failed: run run-f under {under} reads running with no driver (no \
             descriptor); resume it with: brokkr resume --run=run-f --db={}",
            db.display()
        ),
        format!("left for reconciliation: run run-l under {under}"),
        format!(
            "reclaimed: {under}, counted compile (2 since its release): compile: it does not parse"
        ),
        format!("reclaimed: {under}, uncounted: start contended"),
        format!("held: {under} after 3 counted refusals: {escaped}"),
        format!("claim refused: entry 5: {hold}"),
        format!("contended: the judging of the pass: {busy}"),
        format!("contended: the claim of entry 6: {busy}"),
        format!("contended: the spawn row of {under}: {busy}"),
        "queue: 7 waiting, 8 held".to_string(),
    ];
    let said = passed(&every_fact(claim), &db);
    assert_eq!(said.lines().collect::<Vec<_>>(), expected);
    assert!(!said.contains('\u{1b}'), "{said}");
}

/// The spawn-failure line a whole pass prints, after the safe renderer,
/// for a start of `run` in `journal` whose driver could not be spawned.
fn failed_spawn(claim: Claim, run: &str, journal: &Path) -> String {
    let report = Passed {
        started: vec![Started {
            entry: claim.parts().0,
            claim,
            run: run.into(),
            spawned: Err(SpawnFailed::Refused("exec refused".into())),
        }],
        ..Passed::default()
    };
    let said = passed(&report, journal);
    let line = said.lines().find(|line| line.starts_with("spawn failed: "));
    line.unwrap().to_string()
}

/// R3: the spawn failure's line, as the pass prints it, gives the command
/// that resumes the run, spelled from `ResumeArgs`, and clap parses it as a
/// shell splits it. That holds for a journal whose path holds a space and a
/// quote, and for a run id the engine spells with a leading hyphen (a
/// feature with no letter or digit), which only an attached value carries.
#[test]
fn a_spawn_failures_resume_command_parses_as_the_shell_splits_it() {
    let OneClaim { dir, claim, .. } = one_claim("{}");
    let db = dir.path().join("the operator's journals/forge.db");
    for run in ["run-f", "-33fdff61"] {
        let said = failed_spawn(claim, run, &db);
        let (_, command) = said.split_once("resume it with: ").unwrap();
        // The shell's own split: each word it reads, ended by a NUL.
        let split = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("printf '%s\\0' {command}"))
            .output()
            .unwrap();
        assert!(split.status.success(), "{command}");
        let text = String::from_utf8(split.stdout).unwrap();
        let words = text.split_terminator('\0');
        let Cmd::Resume(args) = Cli::try_parse_from(words).unwrap().command else {
            panic!("{command} parses as resume");
        };
        assert_eq!(
            (args.run.as_str(), args.journal.db.as_deref()),
            (run, Some(db.as_path()))
        );
    }
}

/// A journal path or run id that the safe renderer would alter is never
/// printed as the command's value, since the pasted command would then name
/// another journal, which `resume` may create:
/// - a path holding a newline or a reordering mark;
/// - a path that is not UTF-8;
/// - a run id holding a control character.
///
/// The line says why instead.
#[test]
fn a_journal_or_run_this_line_cannot_print_is_never_offered_as_a_command() {
    use std::os::unix::ffi::OsStrExt;
    let OneClaim { dir, claim, .. } = one_claim("{}");
    let root = dir.path();
    let cases = [
        ("run-f", root.join("forge\n.db")),
        ("run-f", root.join("forge\u{202E}bd.db")),
        (
            "run-f",
            root.join(std::ffi::OsStr::from_bytes(b"forge\xff.db")),
        ),
        ("run\u{7}f", root.join("forge.db")),
    ];
    for (run, journal) in &cases {
        let said = failed_spawn(claim, run, journal);
        assert!(
            said.ends_with(
                "resume it with brokkr resume, naming the run and this journal by hand: \
                 their spelling holds characters this line cannot print"
            ),
            "{said:?}"
        );
        assert!(!said.contains("--db"), "{said:?}");
    }
}

/// A pass that ends in its reconciliation's spawns keeps every linked run
/// it did not hand to a driver (decision 0110 ruling 5, part A's A1), and
/// its lines name each by run and claim, the one it handed once.
#[test]
fn an_ending_names_every_reconciled_run_its_driver_did_not_reach() {
    let OneClaim {
        db,
        claim,
        mut _held,
        ..
    } = one_claim("{}");
    let (store, dispatcher) = &mut _held;
    // Each claim linked to its run before the next is taken, as the
    // reconciliation finds them: linked, with no `spawn` row.
    let link = |store: &mut brokkr_store::Store, claim, run: &str| {
        let manifest = serde_json::json!({});
        store.create_run(run, "f", "b", &manifest).unwrap();
        let generation = dispatcher.generation;
        store.queue_link(claim, generation, run, "r").unwrap();
    };
    link(store, claim, "run-a");
    let second = claimed_by(store, dispatcher, "{}");
    link(store, second, "run-b");
    let third = claimed_by(store, dispatcher, "{}");
    let ended = Ended {
        ending: Ending::Replaced(Path::new("/opt/brokkr").into()),
        passed: Passed {
            reconciled: vec![
                Settled::ToSpawn(claim, "run-a".into()),
                Settled::ToSpawn(second, "run-b".into()),
                Settled::ToSpawn(third, "run-c".into()),
            ],
            spawned: vec![Spawning {
                claim,
                run: "run-a".into(),
                spawned: Ok(Spawned { pid: 9 }),
            }],
            ..Passed::default()
        },
    };
    let said = super::ended(&ended, &db);
    let pending = |under: Claim, run: &str| {
        format!(
            "reconciled: run {run} under {} is linked with no driver yet; the next pass spawns it",
            claimed(under)
        )
    };
    let expected = [
        pending(second, "run-b"),
        pending(third, "run-c"),
        format!("started: run run-a under {}, driver pid 9", claimed(claim)),
        "queue: 0 waiting, 0 held".to_string(),
        format!("ended: {}", ended.ending),
    ];
    assert_eq!(said.lines().collect::<Vec<_>>(), expected);
    assert_ne!(second, third, "two claims of their own");
}

#[test]
fn every_step_and_subject_is_named() {
    let named = [
        Stage::Reconcile,
        Stage::Sweep,
        Stage::Bound,
        Stage::Judge,
        Stage::Start,
        Stage::Claim,
        Stage::Link,
        Stage::Spawn,
    ]
    .map(stage_word);
    assert_eq!(
        named,
        [
            "the reconciliation",
            "the sweep",
            "the bound's read",
            "the judging",
            "the start",
            "the claim",
            "the link",
            "the spawn row"
        ]
    );
}

#[test]
fn an_ending_prints_what_the_pass_did_then_why_it_ended() {
    let dir = tempfile::tempdir().unwrap();
    let journal = dir.path().join("forge.db");
    let refused = SweepRefused {
        report: Swept::default(),
        cause: "disk full".into(),
    };
    let ended = Ended {
        ending: Ending::Replaced(Path::new("/opt/brokkr").into()),
        passed: Passed {
            sweep: Some(SweepOutcome::Refused(refused)),
            ..Passed::default()
        },
    };
    assert_eq!(
        super::ended(&ended, &journal),
        "sweep: freed 0 bytes\nsweep: the removals above were not recorded: disk full\nqueue: 0 \
         waiting, 0 held\nended: dispatch: the brokkr at /opt/brokkr is not the one this \
         dispatcher started from; it claimed nothing after; start the dispatcher again\n"
    );
}
