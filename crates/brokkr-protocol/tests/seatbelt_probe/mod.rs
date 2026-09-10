//! Bounded R3 lifetime feasibility probe for the per-invocation transient
//! launchd lease pair (decision 0046 slice II, design D2/D3/D14).
//!
//! This is experimental probe scaffolding, not production Seatbelt code.
//! The shared model below is ordinary Rust, compiled and exercised on every
//! host through the [`ProbeHost`] seam; the macOS adapter in [`native`]
//! performs the real launchd experiment and is compiled only on macOS. A
//! passing model in a Linux test is NOT native evidence: SEATBELT-R3 stays
//! open until the real macOS cases run and pass, and every failure arm below
//! is a failure, never a skip.
//!
//! Design D3 splits admission into two gates:
//!
//! 1. **Gate A — startup**: four cells S0–S3 over direct/launchd ownership and
//!    the exact candidate Seatbelt profile off/on. All four must reach a
//!    nonce-authenticated READY, identify an ordinary child and exit cleanly
//!    on identical helper bytes and argv before any lifetime trigger.
//! 2. **Gate B — lifetime**: the adversarial matrix. It is not run at all
//!    unless Gate A passed for the same helper/profile candidate.
//!
//! The model records the measurement-repair facts the controller findings
//! require (design D3): the real trigger each case performs, attack recording
//! before observation, guard liveness sampled while still registered, a real
//! original-process-group kill, public start identities, preserved failure
//! reports, and the ban on payload-forged markers or harness cleanup being
//! counted as containment.

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

    /// The one real, case-specific trigger this case must perform. Naming a
    /// shared detach routine for several cases is a measurement defect.
    pub fn required_trigger(self) -> TriggerKind {
        use Case::*;
        match self {
            OrdinaryChild => TriggerKind::OrdinaryCompletion,
            Timeout => TriggerKind::Timeout,
            Cancellation => TriggerKind::Cancellation,
            SupervisorDeath => TriggerKind::SupervisorKill,
            DoubleFork => TriggerKind::DoubleFork,
            IgnoredSignals => TriggerKind::IgnoredSignal,
            RetainedPipes => TriggerKind::RetainedPipe,
            ParentExit => TriggerKind::ParentExit,
            GuardInterference => TriggerKind::GuardAttack,
            PeerBootout => TriggerKind::PeerAttack,
            EscapeJob => TriggerKind::EscapeRegistration,
            GroupKillNegativeControl => TriggerKind::OriginalGroupKill,
        }
    }

    /// The complete lifecycle a completed case must record, matched exactly:
    /// an intended-event prefix is not a pass.
    pub fn expected_lifecycle(self) -> Vec<Event> {
        if self == Case::GroupKillNegativeControl {
            // The negative control is independent of the guard and liveness
            // channel, so it never registers or unregisters a guard.
            vec![
                Event::Prepared,
                Event::PayloadStarted,
                Event::Terminating,
                Event::PayloadQuiescent,
                Event::PrivateStateRemoved,
            ]
        } else {
            LIFECYCLE.to_vec()
        }
    }

    /// Whether this case observes a registered guard before releasing
    /// cleanup (everything except the independent negative control).
    pub fn observes_guard(self) -> bool {
        self != Case::GroupKillNegativeControl
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

/// The distinct real trigger each lifetime case performs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerKind {
    OrdinaryCompletion,
    Timeout,
    Cancellation,
    SupervisorKill,
    DoubleFork,
    IgnoredSignal,
    RetainedPipe,
    ParentExit,
    GuardAttack,
    PeerAttack,
    EscapeRegistration,
    OriginalGroupKill,
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

/// The measurement-repair facts task 1.7 requires. Each is a recorded
/// observation, never a source-reasoning claim; the injected tests forge one
/// at a time and demand a failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepairFacts {
    /// The real trigger this case performed; must equal the case's requirement.
    pub trigger: TriggerKind,
    /// A denial case durably recorded the attempted attack before observing
    /// that it was denied.
    pub attack_recorded_before_observation: bool,
    /// The peer job was synchronized to READY before the payload attacked it.
    pub peer_synchronized_before_attack: bool,
    /// Guard liveness was sampled while the guard was still registered.
    pub guard_observed_registered: bool,
    /// The negative control sent a real `SIGKILL` to the original process
    /// group (never the observer's own group).
    pub real_original_group_kill: bool,
    /// Every reported PID was paired with a public start identity so PID reuse
    /// cannot be mistaken for survival.
    pub identities_have_start_time: bool,
    /// Every holder, helper and child was waited or reaped on this path.
    pub reaped_all_children: bool,
    /// A failed case preserved its durable report before harness cleanup.
    pub failure_report_preserved: bool,
    /// The payload was able to write evidence the observer/guard relied on.
    pub forged_marker: bool,
    /// Harness cleanup was counted as containment evidence.
    pub harness_cleanup_counted: bool,
}

impl RepairFacts {
    /// The passing facts for a case, so tests vary exactly one.
    pub fn passing(case: Case) -> RepairFacts {
        RepairFacts {
            trigger: case.required_trigger(),
            attack_recorded_before_observation: case.expectation() == Expectation::PayloadDenied,
            peer_synchronized_before_attack: case == Case::PeerBootout,
            guard_observed_registered: case.observes_guard(),
            real_original_group_kill: case == Case::GroupKillNegativeControl,
            identities_have_start_time: true,
            reaped_all_children: true,
            failure_report_preserved: true,
            forged_marker: false,
            harness_cleanup_counted: false,
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
    /// The task 1.7 measurement-repair facts.
    pub repairs: RepairFacts,
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
            repairs: RepairFacts::passing(case),
        }
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
            Verdict::Pass => "PASS".to_string(),
            Verdict::Fail(reasons) => {
                let mut out = String::from("FAIL");
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
/// last. Exact per-case equality is enforced by [`evaluate_case`].
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
    evaluate_repairs(result, reasons);
    let wanted = result.case.expected_lifecycle();
    if result.events != wanted {
        reasons.push(format!(
            "{name}: lifecycle {:?} is not the exact required {:?}",
            result.events, wanted
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

/// Reject forged or harness-contaminated measurement, a wrong trigger, or a
/// lost failure report before a case's guarantee is even considered.
fn evaluate_repairs(result: &CaseResult, reasons: &mut Vec<String>) {
    let name = result.case.name();
    if result.repairs.trigger != result.case.required_trigger() {
        reasons.push(format!(
            "{name}: performed {:?}, not the required {:?} trigger",
            result.repairs.trigger,
            result.case.required_trigger()
        ));
    }
    if result.repairs.forged_marker {
        reasons.push(format!(
            "{name}: payload-writable state was used as guard/observer evidence"
        ));
    }
    if result.repairs.harness_cleanup_counted {
        reasons.push(format!(
            "{name}: harness cleanup was counted as containment evidence"
        ));
    }
    if !result.repairs.identities_have_start_time {
        reasons.push(format!(
            "{name}: a reported PID had no public start identity"
        ));
    }
    if !result.repairs.reaped_all_children {
        reasons.push(format!(
            "{name}: a holder, helper or child was abandoned rather than reaped"
        ));
    }
    if !result.repairs.failure_report_preserved {
        reasons.push(format!("{name}: a failure lost its durable report"));
    }
    if result.case.observes_guard() && !result.repairs.guard_observed_registered {
        reasons.push(format!(
            "{name}: guard liveness was sampled only after unregister"
        ));
    }
    if result.case.expectation() == Expectation::PayloadDenied
        && !result.repairs.attack_recorded_before_observation
    {
        reasons.push(format!(
            "{name}: the attack was observed without a prior recorded attempt"
        ));
    }
    if result.case == Case::PeerBootout && !result.repairs.peer_synchronized_before_attack {
        reasons.push(format!(
            "{name}: the peer was not synchronized before the attack"
        ));
    }
    if result.case == Case::GroupKillNegativeControl && !result.repairs.real_original_group_kill {
        reasons.push(format!(
            "{name}: no real original-process-group SIGKILL was performed"
        ));
    }
}

/// The operations the shared probe driver needs from a host. The macOS
/// adapter performs them with public launchd facilities; tests supply
/// scripted facts.
pub trait ProbeHost {
    /// The first failed public prerequisite, or `None` when the host can run
    /// the probe. Checked before any payload executes.
    fn precondition(&mut self) -> Option<Precondition>;

    /// Run one Gate A startup cell to its observation.
    fn run_startup_cell(&mut self, cell: StartupCell) -> StartupObservation;

    /// Run one Gate B lifetime case to its trigger and observation.
    fn run_case(&mut self, case: Case) -> CaseResult;
}

/// One probe run's selection, observations and verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeReport {
    pub selected: Vec<Case>,
    pub results: Vec<CaseResult>,
    pub verdict: Verdict,
}

/// Drive the lifetime matrix: check the precondition, run every selected
/// case, then evaluate. A failed precondition runs no case.
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

// ---------------------------------------------------------------------------
// Gate A: the S0-S3 startup matrix (design D3)
// ---------------------------------------------------------------------------

/// The four startup cells: launch ownership (direct observer vs. transient
/// launchd payload job) crossed with the candidate Seatbelt profile (off/on).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StartupCell {
    /// Outside observer, no Seatbelt.
    S0DirectUnboxed,
    /// Outside observer, exact candidate profile.
    S1DirectSeatbelt,
    /// Transient launchd payload job, no Seatbelt.
    S2LaunchdUnboxed,
    /// Transient launchd payload job, exact candidate profile.
    S3LaunchdSeatbelt,
}

impl StartupCell {
    pub const ALL: [StartupCell; 4] = [
        StartupCell::S0DirectUnboxed,
        StartupCell::S1DirectSeatbelt,
        StartupCell::S2LaunchdUnboxed,
        StartupCell::S3LaunchdSeatbelt,
    ];

    pub fn name(self) -> &'static str {
        match self {
            StartupCell::S0DirectUnboxed => "S0-direct-unboxed",
            StartupCell::S1DirectSeatbelt => "S1-direct-seatbelt",
            StartupCell::S2LaunchdUnboxed => "S2-launchd-unboxed",
            StartupCell::S3LaunchdSeatbelt => "S3-launchd-seatbelt",
        }
    }

    /// Whether the helper is launched by a transient launchd job.
    pub fn launchd(self) -> bool {
        matches!(
            self,
            StartupCell::S2LaunchdUnboxed | StartupCell::S3LaunchdSeatbelt
        )
    }

    /// Whether the exact candidate Seatbelt profile is applied.
    pub fn seatbelt(self) -> bool {
        matches!(
            self,
            StartupCell::S1DirectSeatbelt | StartupCell::S3LaunchdSeatbelt
        )
    }
}

impl fmt::Display for StartupCell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// How the helper process ended in a startup cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelperExit {
    /// Exited zero before an outer timeout.
    Clean,
    /// Exited nonzero with this code.
    NonZero(i32),
    /// Ended on this signal.
    Signal(i32),
    /// Never ran to an observed exit.
    NotRun,
}

impl HelperExit {
    pub fn is_clean(self) -> bool {
        matches!(self, HelperExit::Clean)
    }

    pub fn describe(self) -> String {
        match self {
            HelperExit::Clean => "exit 0".to_string(),
            HelperExit::NonZero(code) => format!("exit {code}"),
            HelperExit::Signal(signal) => format!("signal {signal}"),
            HelperExit::NotRun => "did not run".to_string(),
        }
    }
}

/// One startup cell's observation. `helper_digest`, `profile_digest` and
/// `helper_argv` are immutable launch inputs; `ready`, `ordinary_child` and
/// `exit` are the external observations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartupObservation {
    pub cell: StartupCell,
    pub ran: bool,
    pub skip: Option<String>,
    /// Digest of the helper bytes actually executed.
    pub helper_digest: String,
    /// Digest of the exact candidate profile when this cell applies it.
    pub profile_digest: Option<String>,
    /// The argv handed to the helper itself (without any launcher prefix).
    pub helper_argv: Vec<String>,
    /// A READY nonce matching the externally chosen nonce was observed.
    pub ready: bool,
    /// A real ordinary child was spawned and identified.
    pub ordinary_child: bool,
    /// The observed helper exit.
    pub exit: HelperExit,
    /// launchd's `runs` counter for launchd-owned cells.
    pub launchd_runs: Option<u32>,
    /// launchd's `successive crashes` counter for launchd-owned cells.
    pub launchd_crashes: Option<u32>,
    /// A labelled diagnostic (for example an `allow default` profile). It is
    /// recorded for diagnosis and can never make the cell pass.
    pub diagnostic: Option<String>,
    /// Bounded stdout captured for this cell.
    pub stdout: String,
    /// Bounded stderr captured for this cell.
    pub stderr: String,
}

impl StartupObservation {
    /// A cell that did not run, for `skip` reasons.
    pub fn skipped(cell: StartupCell, reason: &str) -> StartupObservation {
        StartupObservation {
            cell,
            ran: false,
            skip: Some(reason.to_string()),
            helper_digest: String::new(),
            profile_digest: None,
            helper_argv: Vec::new(),
            ready: false,
            ordinary_child: false,
            exit: HelperExit::NotRun,
            launchd_runs: None,
            launchd_crashes: None,
            diagnostic: None,
            stdout: String::new(),
            stderr: String::new(),
        }
    }

    /// Mark this observation as a labelled diagnostic. It can never pass.
    pub fn labelled_diagnostic(mut self, label: &str) -> StartupObservation {
        self.diagnostic = Some(label.to_string());
        self
    }
}

/// The Gate A report: selected cells, observations and typed verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartupReport {
    pub selected: Vec<StartupCell>,
    pub cells: Vec<StartupObservation>,
    pub verdict: Verdict,
}

/// Evaluate the startup matrix. Every selected cell must reach an
/// authenticated READY, identify an ordinary child and exit cleanly, and all
/// selected cells must have used identical helper bytes and helper argv (and
/// identical profile bytes where the profile applies). Any single cell's
/// failure is named separately.
pub fn evaluate_startup(selected: &[StartupCell], cells: &[StartupObservation]) -> Verdict {
    let mut reasons = Vec::new();
    if selected.is_empty() {
        reasons.push("zero startup cells selected: admission cannot be proven".to_string());
        return Verdict::Fail(reasons);
    }
    let mut helper_digest: Option<&str> = None;
    let mut helper_argv: Option<&[String]> = None;
    let mut profile_digest: Option<&str> = None;
    for cell in selected {
        let Some(observation) = cells.iter().find(|candidate| candidate.cell == *cell) else {
            reasons.push(format!(
                "{}: selected but produced no observation",
                cell.name()
            ));
            continue;
        };
        evaluate_startup_cell(observation, &mut reasons);
        if !observation.ran {
            continue;
        }
        match helper_digest {
            None => helper_digest = Some(&observation.helper_digest),
            Some(first) if first != observation.helper_digest => reasons.push(format!(
                "{}: helper bytes differ from the first selected cell",
                cell.name()
            )),
            Some(_) => {}
        }
        match helper_argv {
            None => helper_argv = Some(&observation.helper_argv),
            Some(first) if first != observation.helper_argv.as_slice() => reasons.push(format!(
                "{}: helper argv differs from the first selected cell",
                cell.name()
            )),
            Some(_) => {}
        }
        if cell.seatbelt() {
            match profile_digest {
                None => profile_digest = Some(observation.profile_digest.as_deref().unwrap_or("")),
                Some(first) if Some(first) != observation.profile_digest.as_deref() => reasons
                    .push(format!(
                        "{}: profile bytes differ from the first Seatbelt cell",
                        cell.name()
                    )),
                Some(_) => {}
            }
        }
    }
    match reasons.is_empty() {
        true => Verdict::Pass,
        false => Verdict::Fail(reasons),
    }
}

fn evaluate_startup_cell(observation: &StartupObservation, reasons: &mut Vec<String>) {
    let name = observation.cell.name();
    if let Some(label) = &observation.diagnostic {
        // A diagnostic profile (for example `allow default`) is admissible
        // only as a labelled, non-passing diagnostic: it names what was tried
        // and can never be counted as startup admission.
        reasons.push(format!(
            "{name}: labelled non-passing diagnostic, not an admission: {label}"
        ));
        return;
    }
    if !observation.ran {
        let why = observation
            .skip
            .as_deref()
            .unwrap_or("did not run and named no reason");
        reasons.push(format!("{name}: startup cell did not run: {why}"));
        return;
    }
    if observation.helper_digest.is_empty() {
        reasons.push(format!("{name}: no helper digest was recorded"));
    }
    if observation.cell.seatbelt() && observation.profile_digest.is_none() {
        reasons.push(format!("{name}: Seatbelt cell recorded no profile digest"));
    }
    if !observation.ready {
        reasons.push(format!(
            "{name}: the helper never reached a nonce-authenticated READY"
        ));
    }
    if !observation.ordinary_child {
        reasons.push(format!("{name}: no ordinary child was identified"));
    }
    if !observation.exit.is_clean() {
        reasons.push(format!(
            "{name}: the helper did not exit cleanly ({})",
            observation.exit.describe()
        ));
    }
    if observation.cell.launchd() {
        match (observation.launchd_runs, observation.launchd_crashes) {
            (Some(runs), Some(0)) if runs >= 1 => {}
            (runs, crashes) => reasons.push(format!(
                "{name}: launchd run/crash facts are not one clean run (runs={runs:?}, crashes={crashes:?})"
            )),
        }
    }
}

/// Run the selected startup cells.
pub fn run_startup(host: &mut dyn ProbeHost, selected: &[StartupCell]) -> StartupReport {
    let precondition = host.precondition();
    let mut cells = Vec::new();
    if precondition.is_none() && !selected.is_empty() {
        for cell in selected {
            cells.push(host.run_startup_cell(*cell));
        }
    }
    let verdict = match precondition.as_ref() {
        Some(precondition) => Verdict::Fail(vec![format!(
            "startup precondition failed: {}",
            precondition.describe()
        )]),
        None => evaluate_startup(selected, &cells),
    };
    StartupReport {
        selected: selected.to_vec(),
        cells,
        verdict,
    }
}

// ---------------------------------------------------------------------------
// The two gates in sequence (design D3/D14)
// ---------------------------------------------------------------------------

/// The gated report: Gate A always runs; Gate B exists only when Gate A
/// passed. A `None` lifetime report means the lifetime matrix was not run,
/// which is a failure, never a pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatedReport {
    pub startup: StartupReport,
    pub lifetime: Option<ProbeReport>,
    pub verdict: Verdict,
}

/// Run Gate A, then Gate B only if Gate A passed. The overall verdict names a
/// startup failure separately from a lifetime failure.
pub fn run_gated_probe(
    host: &mut dyn ProbeHost,
    startup_selected: &[StartupCell],
    lifetime_selected: &[Case],
) -> GatedReport {
    let startup = run_startup(host, startup_selected);
    if !startup.verdict.is_pass() {
        return GatedReport {
            startup,
            lifetime: None,
            verdict: Verdict::Fail(vec![
                "lifetime matrix not run: the S0-S3 startup gate did not pass".to_string(),
            ]),
        };
    }
    let lifetime = run_probe(host, lifetime_selected);
    let verdict = lifetime.verdict.clone();
    GatedReport {
        startup,
        lifetime: Some(lifetime),
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
        events: case.expected_lifecycle(),
        repairs: RepairFacts::passing(case),
    }
}

/// Construct a passing startup observation for a cell, so tests vary exactly
/// one fact.
pub fn passing_startup(cell: StartupCell) -> StartupObservation {
    StartupObservation {
        cell,
        ran: true,
        skip: None,
        helper_digest: "00ff".to_string(),
        profile_digest: cell.seatbelt().then(|| "aa55".to_string()),
        helper_argv: vec![
            "startup".to_string(),
            "--exit".to_string(),
            "clean".to_string(),
        ],
        ready: true,
        ordinary_child: true,
        exit: HelperExit::Clean,
        launchd_runs: cell.launchd().then_some(1),
        launchd_crashes: cell.launchd().then_some(0),
        diagnostic: None,
        stdout: String::new(),
        stderr: String::new(),
    }
}

#[cfg(test)]
mod tests;
