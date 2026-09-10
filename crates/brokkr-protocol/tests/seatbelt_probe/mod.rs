//! Bounded R3 lifetime feasibility probe for the per-invocation transient
//! launchd lease pair (decision 0046 slice II, design D2/D3).
//!
//! This is experimental probe scaffolding, not production Seatbelt code.
//! The shared model below is ordinary Rust, compiled and exercised on every
//! host through the [`ProbeHost`] seam; the macOS adapter in [`native`]
//! performs the real launchd experiment and is compiled only on macOS. A
//! passing model in a Linux test is NOT native evidence: SEATBELT-R3 stays
//! open until the real macOS cases run and pass, and every failure arm below
//! is a failure, never a skip.

// The adapter is type-checked on every host so a macOS-only compile error
// cannot hide in code this Linux controller cannot run. Its required test is
// macOS-only, and on other hosts its dead code is allowed on purpose.
pub mod native;

use std::fmt;

/// The adversarial families the R3 probe must exercise. The list is the
/// checked obligation matrix: a candidate that runs a subset fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Case {
    /// The positive control: an ordinary child runs and completes.
    OrdinaryChild,
    /// A `setsid` descendant races a workspace deadline.
    Timeout,
    /// A `setsid` descendant races an explicit cancellation.
    Cancellation,
    /// A `setsid` descendant outlives a supervisor `SIGKILL`.
    SupervisorDeath,
    /// A double-fork descendant outlives its immediate parent.
    DoubleFork,
    /// A descendant ignores ordinary termination signals.
    IgnoredSignals,
    /// A descendant retains the payload output pipes.
    RetainedPipes,
    /// The direct parent exits while a background child remains.
    ParentExit,
    /// The payload tries to signal or impersonate the guard.
    GuardInterference,
    /// The payload tries to boot out a peer invocation.
    PeerBootout,
    /// The payload tries to register an independently surviving job.
    EscapeJob,
    /// The process-group-only negative control must leave the helper alive.
    GroupKillNegativeControl,
}

impl Case {
    /// Every case, in the order the probe reports them.
    pub const ALL: [Case; 12] = [
        Case::OrdinaryChild,
        Case::Timeout,
        Case::Cancellation,
        Case::SupervisorDeath,
        Case::DoubleFork,
        Case::IgnoredSignals,
        Case::RetainedPipes,
        Case::ParentExit,
        Case::GuardInterference,
        Case::PeerBootout,
        Case::EscapeJob,
        Case::GroupKillNegativeControl,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Case::OrdinaryChild => "ordinary-child",
            Case::Timeout => "timeout",
            Case::Cancellation => "cancellation",
            Case::SupervisorDeath => "supervisor-death",
            Case::DoubleFork => "double-fork",
            Case::IgnoredSignals => "ignored-signals",
            Case::RetainedPipes => "retained-pipes",
            Case::ParentExit => "parent-exit",
            Case::GuardInterference => "guard-interference",
            Case::PeerBootout => "peer-bootout",
            Case::EscapeJob => "escape-job",
            Case::GroupKillNegativeControl => "group-kill-negative-control",
        }
    }

    /// What a pass requires of this case's observation.
    pub fn expectation(self) -> Expectation {
        use Case::*;
        match self {
            OrdinaryChild => Expectation::OrdinaryCompletion,
            Timeout | Cancellation | SupervisorDeath | DoubleFork | IgnoredSignals
            | RetainedPipes | ParentExit => Expectation::NoSurvivor,
            GuardInterference | PeerBootout | EscapeJob => Expectation::PayloadDenied,
            GroupKillNegativeControl => Expectation::GroupKillLeavesSurvivor,
        }
    }
}

impl fmt::Display for Case {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The success shape of a case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expectation {
    /// The ordinary payload completed and its positive control fired.
    OrdinaryCompletion,
    /// The payload left no surviving descendant, moving heartbeat or label.
    NoSurvivor,
    /// An original-process-group-only kill left the helper alive, proving the
    /// adversary actually detached.
    GroupKillLeavesSurvivor,
    /// The payload's interference/escape attempt was denied and the guard
    /// survived the payload's teardown.
    PayloadDenied,
}

/// The ordered lifecycle the design requires. A missing, repeated or
/// reordered event is a failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Prepared,
    GuardRegistered,
    PayloadStarted,
    Terminating,
    PayloadQuiescent,
    PrivateStateRemoved,
    GuardUnregistered,
}

/// The required lifecycle order (design D2).
pub const LIFECYCLE: [Event; 7] = [
    Event::Prepared,
    Event::GuardRegistered,
    Event::PayloadStarted,
    Event::Terminating,
    Event::PayloadQuiescent,
    Event::PrivateStateRemoved,
    Event::GuardUnregistered,
];

/// A public prerequisite outside the probe's control.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Precondition {
    /// The literal trusted `/usr/bin/sandbox-exec` is absent.
    LauncherMissing,
    /// No public per-user `launchctl` is available.
    LaunchctlMissing,
    /// The probe is already inside a Brokkr hands box.
    OuterBox,
    /// A required public facility exists but is unusable for the named reason.
    FacilityUnsupported(String),
}

impl Precondition {
    pub fn describe(&self) -> String {
        match self {
            Precondition::LauncherMissing => {
                "the literal /usr/bin/sandbox-exec is absent".to_string()
            }
            Precondition::LaunchctlMissing => {
                "no public per-user launchctl is available".to_string()
            }
            Precondition::OuterBox => "the probe is already inside a Brokkr hands box".to_string(),
            Precondition::FacilityUnsupported(reason) => {
                format!("a required public facility is unusable: {reason}")
            }
        }
    }
}

/// One case's observation. Every field is a fact the outside observer or the
/// guard recorded; none may be inferred from source reasoning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseResult {
    pub case: Case,
    /// The case actually ran to the trigger and observation.
    pub ran: bool,
    /// Why it did not run; a skip is a failure, never a pass.
    pub skip: Option<String>,
    /// The ordinary payload or adversarial heartbeat advanced before trigger.
    pub positive_control: bool,
    /// A heartbeat was seen advancing before the trigger.
    pub heartbeat_before: bool,
    /// Live payload identities after the trigger and five-second bound.
    pub survivors_after: usize,
    /// The heartbeat moved during the further one-second quiet window.
    pub heartbeat_moved_during_quiet: bool,
    /// Both transient launchd job labels are absent.
    pub labels_gone: bool,
    /// Private state was removed only after payload quiescence.
    pub cleanup_after_quiescence: bool,
    /// The guard outlived the payload `bootout`.
    pub guard_survived_payload: bool,
    /// A denial case's interference/escape attempt was refused.
    pub payload_attempt_denied: bool,
    /// The lifecycle events the host recorded, in order.
    pub events: Vec<Event>,
}

impl CaseResult {
    /// A result that did not run, for `skip` reasons.
    pub fn skipped(case: Case, reason: &str) -> CaseResult {
        CaseResult {
            case,
            ran: false,
            skip: Some(reason.to_string()),
            positive_control: false,
            heartbeat_before: false,
            survivors_after: 0,
            heartbeat_moved_during_quiet: false,
            labels_gone: false,
            cleanup_after_quiescence: false,
            guard_survived_payload: false,
            payload_attempt_denied: false,
            events: Vec::new(),
        }
    }

    /// The full ordered lifecycle a completed case records.
    fn complete_events() -> Vec<Event> {
        LIFECYCLE.to_vec()
    }
}

/// The probe's verdict: `Pass` only when every selected case's expectation
/// holds. Any `Fail` names every unmet obligation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Pass,
    Fail(Vec<String>),
}

impl Verdict {
    pub fn is_pass(&self) -> bool {
        matches!(self, Verdict::Pass)
    }

    pub fn reasons(&self) -> &[String] {
        match self {
            Verdict::Pass => &[],
            Verdict::Fail(reasons) => reasons,
        }
    }

    pub fn render(&self) -> String {
        match self {
            Verdict::Pass => "R3 lifetime feasibility probe: PASS".to_string(),
            Verdict::Fail(reasons) => {
                let mut out = String::from("R3 lifetime feasibility probe: FAIL");
                for reason in reasons {
                    out.push_str("\n  - ");
                    out.push_str(reason);
                }
                out
            }
        }
    }
}

/// Is `events` the required lifecycle in order, possibly a prefix when the
/// case failed early? A repeated, reordered or unknown event is a failure,
/// so cleanup can never precede quiescence and the guard always unregisters
/// last.
pub fn events_are_ordered(events: &[Event]) -> bool {
    let mut next = 0;
    for event in events {
        let Some(position) = LIFECYCLE[next..].iter().position(|step| step == event) else {
            return false;
        };
        next += position + 1;
    }
    true
}

/// Evaluate the probe's observations. Pure, so every failure arm is a unit
/// test rather than a native run.
pub fn evaluate(
    selected: &[Case],
    results: &[CaseResult],
    precondition: Option<&Precondition>,
) -> Verdict {
    let mut reasons = Vec::new();
    if let Some(precondition) = precondition {
        reasons.push(format!(
            "probe precondition failed: {}",
            precondition.describe()
        ));
        return Verdict::Fail(reasons);
    }
    if selected.is_empty() {
        reasons.push("zero cases selected: the obligation matrix is empty".to_string());
        return Verdict::Fail(reasons);
    }
    for case in selected {
        match results.iter().find(|result| result.case == *case) {
            None => reasons.push(format!("{}: selected but produced no result", case.name())),
            Some(result) => evaluate_case(result, &mut reasons),
        }
    }
    match reasons.is_empty() {
        true => Verdict::Pass,
        false => Verdict::Fail(reasons),
    }
}

fn evaluate_case(result: &CaseResult, reasons: &mut Vec<String>) {
    let name = result.case.name();
    if !result.ran {
        let why = result
            .skip
            .as_deref()
            .unwrap_or("did not run and named no reason");
        reasons.push(format!("{name}: skipped or did not run: {why}"));
        return;
    }
    if !events_are_ordered(&result.events) {
        reasons.push(format!(
            "{name}: lifecycle events are missing, repeated or out of order: {:?}",
            result.events
        ));
    }
    match result.case.expectation() {
        Expectation::OrdinaryCompletion => {
            if !result.positive_control {
                reasons.push(format!(
                    "{name}: ordinary-child positive control did not complete"
                ));
            }
            if !result.labels_gone {
                reasons.push(format!("{name}: a transient job label survived"));
            }
        }
        Expectation::NoSurvivor => {
            if !result.positive_control || !result.heartbeat_before {
                reasons.push(format!(
                    "{name}: the adversarial heartbeat never advanced before the trigger"
                ));
            }
            if result.survivors_after != 0 {
                reasons.push(format!(
                    "{name}: {} payload descendant(s) survived the teardown bound",
                    result.survivors_after
                ));
            }
            if result.heartbeat_moved_during_quiet {
                reasons.push(format!(
                    "{name}: the heartbeat moved during the one-second quiet window"
                ));
            }
            if !result.labels_gone {
                reasons.push(format!("{name}: a transient job label survived"));
            }
            if !result.cleanup_after_quiescence {
                reasons.push(format!(
                    "{name}: private state was removed before quiescence"
                ));
            }
        }
        Expectation::GroupKillLeavesSurvivor => {
            if !result.positive_control {
                reasons.push(format!(
                    "{name}: the helper never proved it was running before the group kill"
                ));
            }
            if result.survivors_after == 0 {
                reasons.push(format!(
                    "{name}: the helper did not survive an original-process-group-only kill, \
                     so the probe never exercised the required detach"
                ));
            }
            if !result.cleanup_after_quiescence {
                reasons.push(format!(
                    "{name}: the helper was cleaned up before observation"
                ));
            }
        }
        Expectation::PayloadDenied => {
            if !result.payload_attempt_denied {
                reasons.push(format!(
                    "{name}: payload interference or job registration was not denied"
                ));
            }
            if !result.guard_survived_payload {
                reasons.push(format!(
                    "{name}: the guard did not survive payload teardown"
                ));
            }
        }
    }
}

/// The operations the shared probe driver needs from a host. The macOS
/// adapter performs them with public launchd facilities; tests supply
/// scripted facts.
pub trait ProbeHost {
    /// The first failed public prerequisite, or `None` when the host can run
    /// the probe. Checked before any payload executes.
    fn precondition(&mut self) -> Option<Precondition>;

    /// Run one case to its trigger and observation, recording the lifecycle.
    fn run_case(&mut self, case: Case) -> CaseResult;
}

/// One probe run's selection, observations and verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeReport {
    pub selected: Vec<Case>,
    pub results: Vec<CaseResult>,
    pub verdict: Verdict,
}

/// Drive the probe: check the precondition, run every selected case, then
/// evaluate. A failed precondition runs no case.
pub fn run_probe(host: &mut dyn ProbeHost, selected: &[Case]) -> ProbeReport {
    let precondition = host.precondition();
    let mut results = Vec::new();
    if precondition.is_none() && !selected.is_empty() {
        for case in selected {
            results.push(host.run_case(*case));
        }
    }
    let verdict = evaluate(selected, &results, precondition.as_ref());
    ProbeReport {
        selected: selected.to_vec(),
        results,
        verdict,
    }
}

/// Construct a passing result for a case, so tests can vary exactly one fact.
pub fn passing_result(case: Case) -> CaseResult {
    let (survivors_after, heartbeat_moved, cleanup) = match case.expectation() {
        // The negative control passes only because a survivor *remains*.
        Expectation::GroupKillLeavesSurvivor => (1, false, true),
        _ => (0, false, true),
    };
    CaseResult {
        case,
        ran: true,
        skip: None,
        positive_control: true,
        heartbeat_before: true,
        survivors_after,
        heartbeat_moved_during_quiet: heartbeat_moved,
        labels_gone: true,
        cleanup_after_quiescence: cleanup,
        guard_survived_payload: true,
        payload_attempt_denied: true,
        events: CaseResult::complete_events(),
    }
}

#[cfg(test)]
mod tests;
