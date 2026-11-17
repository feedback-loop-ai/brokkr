//! The lines `brokkr dispatch` prints (decision 0110 rulings 3 and 6, the
//! controller's R3 and R5): one plain line per fact of a pass's report and
//! of the holder's readout, each painted through the same control-character
//! escaping every operator line uses (#468). The facts are brokkr-runtime's
//! and brokkr-store's; only their words are here.

use std::num::NonZeroU32;
use std::path::Path;

use brokkr_core::fold::Status;
use brokkr_runtime::dispatch::{
    BoundOutcome, Contended, Ended, Ending, Exited, Held, Left, Passed, Reaped, Reclaimed, Settled,
    SpawnFailed, Spawned, Spawning, Stage, Started, Subject, SweepOutcome,
};
use brokkr_store::{Claim, Generation};

use crate::render::{shell_word, Safe};

/// The safe renderer every operator line uses (#468).
fn safe(text: &str) -> String {
    Safe::new(text).as_str().to_string()
}

/// `said`, one line, painted safe.
fn line(lines: &mut String, said: &str) {
    lines.push_str(&safe(said));
    lines.push('\n');
}

/// What a pass did, line by line.
pub(super) fn passed(passed: &Passed, journal: &Path) -> String {
    let mut lines = String::new();
    reconciled(&mut lines, passed);
    for Spawning {
        claim,
        run,
        spawned,
    } in &passed.spawned
    {
        line(&mut lines, &started_line(*claim, run, spawned, journal));
    }
    sweep(&mut lines, passed.sweep.as_ref());
    if let Some(BoundOutcome::Unreadable(error)) = &passed.bound {
        line(
            &mut lines,
            &format!("bound unreadable: {error}; nothing was claimed"),
        );
    }
    if let Some(error) = &passed.admission_refused {
        line(
            &mut lines,
            &format!("admission refused: {error}; nothing more was claimed"),
        );
    }
    for reason in &passed.unstarted {
        line(&mut lines, &format!("unstarted: {reason}"));
    }
    entries(&mut lines, passed, journal);
    let (waiting, held) = (passed.waiting, passed.held_entries);
    line(
        &mut lines,
        &format!("queue: {waiting} waiting, {held} held"),
    );
    lines
}

/// What the reconciliation settled: each claim it returned to its place,
/// and each linked run whose driver this pass did not reach, which a pass
/// that ended or met a refused `spawn` row keeps (decision 0110 ruling 5)
/// and the next pass's reconciliation spawns. A run this pass handed to a
/// driver is named once, by its started line.
fn reconciled(lines: &mut String, passed: &Passed) {
    for settled in &passed.reconciled {
        let said = match settled {
            Settled::Reclaimed(claim) => format!(
                "reconciled: {}, which no run carries, returned to its place",
                claimed(*claim)
            ),
            Settled::ToSpawn(claim, run) if !passed.spawned.iter().any(|s| s.claim == *claim) => {
                format!(
                    "reconciled: run {run} under {} is linked with no driver yet; the next \
                     pass spawns it",
                    claimed(*claim)
                )
            }
            Settled::ToSpawn(..) => continue,
        };
        line(lines, &said);
    }
}

/// What each entry's start came to: started, left, reclaimed, held,
/// refused at its claim, or contended.
fn entries(lines: &mut String, passed: &Passed, journal: &Path) {
    for Started {
        claim,
        run,
        spawned,
        ..
    } in &passed.started
    {
        line(lines, &started_line(*claim, run, spawned, journal));
    }
    for Left { claim, run } in &passed.left {
        line(
            lines,
            &format!(
                "left for reconciliation: run {run} under {}",
                claimed(*claim)
            ),
        );
    }
    for Reclaimed {
        claim,
        refused,
        count,
        reason,
    } in &passed.reclaimed
    {
        let counted = match refused {
            Some(refusal) => format!("counted {} ({count} since its release)", refusal.word()),
            None => "uncounted".to_string(),
        };
        line(
            lines,
            &format!("reclaimed: {}, {counted}: {reason}", claimed(*claim)),
        );
    }
    for Held {
        claim,
        count,
        reason,
        ..
    } in &passed.held
    {
        let held = format!(
            "held: {} after {count} counted refusals: {reason}",
            claimed(*claim)
        );
        line(lines, &held);
    }
    for refused in &passed.claims_refused {
        line(
            lines,
            &format!("claim refused: entry {}: {}", refused.entry, refused.reason),
        );
    }
    for Contended {
        subject,
        stage,
        error,
    } in &passed.contended
    {
        let at = format!("{} of {}", stage_word(*stage), subject_words(*subject));
        line(lines, &format!("contended: {at}: {error}"));
    }
}

/// A run handed to its driver, or the spawn that failed and how to resume
/// the run (`ResumeArgs`: `--run` and `--db`).
fn started_line(
    claim: Claim,
    run: &str,
    spawned: &Result<Spawned, SpawnFailed>,
    journal: &Path,
) -> String {
    let under = claimed(claim);
    match spawned {
        Ok(Spawned { pid }) => format!("started: run {run} under {under}, driver pid {pid}"),
        Err(SpawnFailed::Refused(why)) => format!(
            "spawn failed: run {run} under {under} reads running with no driver ({why}); \
             resume it with{}",
            resume(run, journal)
        ),
    }
}

/// The command that resumes `run` in `journal`. Each value is attached to
/// its flag, so a run id the engine spells with a leading hyphen still
/// parses as the value. Each value is also one shell word, so the command
/// pastes as it reads (#820). A run id or journal path that the safe
/// renderer would alter is never printed as a value: one holding a control
/// or reordering character, or a path that is not UTF-8. The line then
/// says why, rather than naming another journal.
fn resume(run: &str, journal: &Path) -> String {
    match journal.to_str() {
        Some(path) if printable(path) && printable(run) => format!(
            ": brokkr resume --run={} --db={}",
            shell_word(run),
            shell_word(path)
        ),
        _ => " brokkr resume, naming the run and this journal by hand: their spelling holds \
              characters this line cannot print"
            .to_string(),
    }
}

/// Whether the safe renderer leaves `text` as it is.
fn printable(text: &str) -> bool {
    Safe::new(text).as_str() == text
}

/// The sweep's own lines, and a record it could not write.
fn sweep(lines: &mut String, outcome: Option<&SweepOutcome>) {
    match outcome {
        Some(SweepOutcome::Swept(swept)) => lines.push_str(&swept.lines_with(safe, false)),
        Some(SweepOutcome::Refused(refused)) => {
            lines.push_str(&refused.report.lines_with(safe, false));
            let cause = &refused.cause;
            line(
                lines,
                &format!("sweep: the removals above were not recorded: {cause}"),
            );
        }
        None => {}
    }
}

/// What a pass did before it ended, then why it ended.
pub(super) fn ended(ended: &Ended, journal: &Path) -> String {
    let mut lines = passed(&ended.passed, journal);
    lines.push_str(&ending(&ended.ending));
    lines
}

/// Why the dispatcher ended: the standing loop's last line once its tick
/// is reported.
pub(super) fn ending(ending: &Ending) -> String {
    let mut lines = String::new();
    line(&mut lines, &format!("ended: {ending}"));
    lines
}

/// A driver the standing loop reaped, its exit and what its run reads
/// now. A run that still reads `running` has no driver, and the line gives
/// the command that resumes it (decision 0110 ruling 5).
pub(super) fn reaped(reaped: &Reaped, journal: &Path) -> String {
    let Exited { run, pid, exit } = &reaped.exited;
    let exited = format!("driver exited: run {run}, driver pid {pid}, {exit}");
    let said = match &reaped.reads {
        Ok(Status::Running) => format!(
            "{exited}; the run reads running with no driver; resume it with{}",
            resume(run, journal)
        ),
        Ok(status) => format!("{exited}; the run reads {}", status.as_str()),
        Err(error) => format!("{exited}; the run does not read: {error}"),
    };
    let mut lines = String::new();
    line(&mut lines, &said);
    lines
}

/// The drivers still running as the standing loop ends: each left
/// running, never signalled.
pub(super) fn running(drivers: &super::Drivers) -> String {
    let mut lines = String::new();
    for (pid, (run, _)) in drivers {
        line(
            &mut lines,
            &format!("left running: run {run}, driver pid {pid}"),
        );
    }
    lines
}

/// The standing loop's words once a signal stopped it.
pub(super) fn stopped() -> String {
    "stopped: a signal asked the dispatcher to stop; it claimed nothing after, and its drivers \
     run on\n"
        .to_string()
}

/// A claim as an operator reads it.
fn claimed(claim: Claim) -> String {
    let (entry, seq) = claim.parts();
    format!("claim {seq} of entry {entry}")
}

/// A pass's step, by the word its line names it with.
fn stage_word(stage: Stage) -> &'static str {
    match stage {
        Stage::Reconcile => "the reconciliation",
        Stage::Sweep => "the sweep",
        Stage::Bound => "the bound's read",
        Stage::Judge => "the judging",
        Stage::Start => "the start",
        Stage::Claim => "the claim",
        Stage::Link => "the link",
        Stage::Spawn => "the spawn row",
    }
}

/// What a contention stood on.
fn subject_words(subject: Subject) -> String {
    match subject {
        Subject::Pass => "the pass".to_string(),
        Subject::Entry(entry) => format!("entry {entry}"),
        Subject::Claim(_, claim) => claimed(claim),
    }
}

/// `brokkr dispatch status`'s lines: whether the lock is `held`, by the
/// latest `holder` the journal records, the `bound` in force, and each
/// entry left launching or linked with no `spawn` row.
pub(super) fn status(
    held: bool,
    holder: Option<&(Generation, u32, String)>,
    bound: NonZeroU32,
    unspawned: &[(Claim, Option<String>)],
) -> String {
    let mut lines = String::new();
    let recorded = holder.map(|(generation, pid, since)| {
        format!(
            "generation {} (pid {pid} on this host since {since})",
            generation.0
        )
    });
    let said = match (held, recorded) {
        (true, Some(recorded)) => format!("held by {recorded}"),
        (true, None) => "held by a dispatcher that has not recorded itself yet".to_string(),
        (false, Some(recorded)) => format!("not held; last held by {recorded}"),
        (false, None) => "not held; no dispatcher has taken this journal".to_string(),
    };
    line(&mut lines, &said);
    line(
        &mut lines,
        &format!("bound: {bound} counted refusals hold an entry"),
    );
    for (claim, run) in unspawned {
        let said = match run {
            Some(run) => format!(
                "linked with no spawn row: run {run} under {}",
                claimed(*claim)
            ),
            None => format!("launching: {}", claimed(*claim)),
        };
        line(&mut lines, &said);
    }
    lines
}

#[cfg(test)]
mod tests;
