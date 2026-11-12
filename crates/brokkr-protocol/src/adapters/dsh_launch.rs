//! The one dsh invocation's lifecycle (#896): its spawn, fold, startup
//! report and commit promotion, moved out of `adapters.rs` when the
//! composed-tree and activation checks grew it past its measured ceiling.

use super::*;

/// The per-invocation transcript fold state both dsh routes share, as one
/// parameter object so a drain call is never a seven-argument list.
pub(super) struct DshFoldState<'a> {
    pub(super) tail: &'a mut DshTail,
    pub(super) turns: &'a mut u64,
    pub(super) session_meta: &'a mut Map<String, Value>,
    pub(super) probe: &'a mut ProbeAnswer,
}

impl DshFoldState<'_> {
    /// Fold whatever the transcript has appended through the invocation's
    /// one probe bound, and return the pre-session refusal it read, if
    /// any. Both routes share this so a pass cannot differ from the other
    /// in which boundary it reads, which refusal it types or which answer
    /// it admits.
    pub(super) fn drain(
        &mut self,
        launch: &DshLaunch,
        bindings: &[secret::BoundSecret],
        emit: &mut impl FnMut(&Value),
    ) -> Option<PreSessionRefusal> {
        DshFold::new(
            self.tail,
            &launch.root,
            launch.first_seq,
            self.turns,
            self.session_meta,
            bindings,
            self.probe,
        )
        .drain(emit)
    }
}

/// `invoke_dsh_launch` with this ONE invocation's observer of the
/// confirmation's completed observations, spawning exactly `command`.
/// Production watches nothing; a test reads here exactly what the watcher
/// consumed — never a second walk of its own — and only once the watcher
/// has ruled on it.
pub(super) fn invoke_dsh_launch_observed(
    mut launch: DshLaunch,
    command: Vec<String>,
    bindings: &[secret::BoundSecret],
    probe: &mut ProbeAnswer,
    emit: &mut impl FnMut(&Value),
    wait: impl FnMut(&mut std::process::Child) -> std::io::Result<Option<i32>>,
    observer: &mut impl FnMut(&DshObservation),
) -> Result<Invocation, String> {
    let workdir = launch.workdir.clone();
    refused_before_spawn(&launch, &workdir, bindings).map_err(|refusal| refusal.to_string())?;
    let transcript = Transcript::dsh(&launch.home);
    let staged = launch.staged.take();
    let mut session_meta = Map::new();
    // Decision 0035 addendum 2026-09-11: a dsh seat with no `--effort`
    // pin compiles only on an effortless route, so the absence IS the
    // standing — no level to forward, none for any header to echo. It
    // is seeded so every fold row and the finishing record read `not
    // applicable` from the first row; a header that echoes a real level
    // overwrites the seed, which is the applied configuration ruling 3
    // asks the fold to read.
    if launch.effortless {
        session_meta.insert(
            "effort".into(),
            Value::String(EFFORT_NOT_APPLICABLE.to_string()),
        );
    }
    // The store is censused BEFORE anything spawns, so a session the
    // plugin opens instead of rejoining the offered one is visible as a
    // session that was not there (task 8.8(d), Pass C).
    let mut watch = DshRootWatch::new(&launch, transcript);
    let mut hold = LaunchHold::new("deepseek", launch.plan());
    // The retained locator is published before anything spawns, exactly as
    // before; `run_seat` holds it until a turn begins (decision 0053). A
    // REJOIN holds it instead: an attempt that never confirms the offered
    // root publishes no locator either, because the row would address a
    // session this driver cannot say it was in (design D7).
    if !watch.confirming() {
        watch.record_locator(&mut hold, &mut session_meta, emit);
    }
    // The shipped cold route confirms nothing, so its one launch row is
    // published before the spawn — exactly where it always was, and the
    // only reason a spawn failure still flushes the held rows. The
    // stream-json route waits for the init event instead.
    if !launch.stream_json {
        hold.finish(emit);
    }
    // Everything the seat committed lives in the private store and
    // nowhere else until the promotion below moves it. Returning early
    // would drop the store — and the seat's work with it — behind an
    // error naming neither (decision 0054 ruling 5).
    let attempt = if launch.stream_json {
        invoke_dsh_stream_json(
            DshStreamCall {
                command: &command,
                launch: &launch,
                bindings,
                probe,
                watch: &mut watch,
                hold: &mut hold,
                session_meta: &mut session_meta,
            },
            emit,
            observer,
            wait,
        )
    } else {
        invoke_dsh_shipped(
            &command,
            &launch,
            bindings,
            wait,
            probe,
            &mut session_meta,
            emit,
        )
    };
    let DshRun {
        mut invocation,
        startup,
    } = match attempt {
        Ok(run) => run,
        Err(problem) => return Err(dsh_failure_before_promotion(problem, staged)),
    };
    if let Some(mode) = launch.mode {
        if let Err(refusal) = dsh_hands::startup_gate(startup, &mut invocation.session_meta, mode) {
            return Err(dsh_failure_before_promotion(refusal.to_string(), staged));
        }
    }
    // The seat wrote its objects and moved its branch inside the private
    // common directory the driver staged; the shared repository was
    // read-only to it throughout. This is where the ONE ref the worktree
    // owns crosses over, outside every box (decision 0054 ruling 5). A
    // promotion that cannot happen is a driver failure with the store's
    // path in it, never a silent loss of the seat's commits.
    if let Some((_, scope, store)) = staged {
        if let Some(promotion) = dsh_sandbox::promote_seat_commits(store, &scope)? {
            invocation.stderr.push_str(&promotion.summary());
            invocation.stderr.push('\n');
        }
    }
    // The qualified route's launch row was held until the harness named
    // its root, or until the invocation ended without one (`finish` is a
    // no-op once a confirmation has already published).
    if launch.stream_json {
        hold.finish(emit);
    }
    invocation.launch = hold.terminal();
    Ok(invocation)
}

/// One qualified invocation's live state: the launch, the root watch, the
/// launch hold, the transcript fold and the probe. A parameter object, so
/// the between-lines poll is one call rather than an eight-argument list
/// (decision 0071 rulings 4 and 10).
struct DshStreamState<'a> {
    launch: &'a DshLaunch,
    bindings: &'a [secret::BoundSecret],
    watch: &'a mut DshRootWatch,
    hold: &'a mut LaunchHold,
    tail: &'a mut DshTail,
    turns: &'a mut u64,
    session_meta: &'a mut Map<String, Value>,
    probe: &'a mut ProbeAnswer,
    /// The pre-session refusal the transcript's first error named, if
    /// any (decision 0053; decision 0103 ruling 1(a)).
    refusal: Option<PreSessionRefusal>,
}

/// What one pass over the qualified child found: an answer, an ordinary
/// tick, or a child that ended.
enum DshPass {
    Answered,
    Continue,
    Exited(i32),
}

impl DshStreamState<'_> {
    /// The watcher's settle with this invocation's own hold and metadata.
    fn settle(
        &mut self,
        line: DshStreamLine,
        emit: &mut impl FnMut(&Value),
        observer: &mut impl FnMut(&DshObservation),
    ) {
        self.watch.settle(
            line,
            &mut *self.hold,
            &mut *self.session_meta,
            emit,
            observer,
        );
    }

    /// Fold whatever the transcript appended, when the hold permits it,
    /// latching the first pre-session refusal it reads. Returns whether
    /// the probe admitted its first answer.
    fn drain(&mut self, emit: &mut impl FnMut(&Value)) -> bool {
        if self.watch.may_fold() {
            let read = DshFoldState {
                tail: &mut *self.tail,
                turns: &mut *self.turns,
                session_meta: &mut *self.session_meta,
                probe: &mut *self.probe,
            }
            .drain(self.launch, self.bindings, emit);
            self.refusal = self.refusal.take().or(read);
        }
        self.probe.answered()
    }

    /// Poll the transcript between stdout lines: settle a held root once
    /// its init event named the offer, then fold at the shipped route's
    /// cadence. An answer never waits for stdout to speak (fix round
    /// 430s4a3 M2; decision 0103 ruling 4(a)).
    fn poll(
        &mut self,
        emit: &mut impl FnMut(&Value),
        observer: &mut impl FnMut(&DshObservation),
    ) -> bool {
        if !self.probe.requested() {
            return false;
        }
        if self.watch.confirming() && self.watch.named_the_offer && !self.watch.settled {
            self.settle(DshStreamLine::Idle, emit, observer);
        }
        self.drain(emit)
    }

    /// One pass over stdout and the transcript. `open` flips false at
    /// stdout's clean end or on a line the reader cannot decode, and a
    /// closed stdout still ticks on the transcript alone: an answer never
    /// waits for stdout to speak (fix round 430s4a3 M2 and M1; decision
    /// 0103 ruling 4(a)).
    fn pass(
        &mut self,
        open: &mut bool,
        reads: &std::sync::mpsc::Receiver<std::io::Result<String>>,
        child: &mut std::process::Child,
        wait: &mut impl FnMut(&mut std::process::Child) -> std::io::Result<Option<i32>>,
        emit: &mut impl FnMut(&Value),
        observer: &mut impl FnMut(&DshObservation),
    ) -> Result<DshPass, std::io::Error> {
        let read = if *open {
            match reads.recv_timeout(DSH_POLL_IDLE) {
                Ok(Ok(line)) => {
                    // Decode a line before the store behind it (design D7).
                    let read = fold_dsh_line(
                        line,
                        &mut *self.watch,
                        &mut *self.hold,
                        &mut *self.session_meta,
                        self.bindings,
                        emit,
                    );
                    // Settle before every drain; never drain while the hold
                    // is closed (design D7, task 7.4).
                    self.settle(read, emit, observer);
                    Some(read)
                }
                Ok(Err(_)) => {
                    self.watch.unread(DshStreamLine::Unreadable);
                    self.settle(DshStreamLine::Unreadable, emit, observer);
                    // The stream ends here, but stdout closing never ends
                    // the transcript's wait (fix round 430s4a3 M1).
                    *open = false;
                    None
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => None,
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    *open = false;
                    None
                }
            }
        } else {
            std::thread::sleep(DSH_POLL_IDLE);
            None
        };
        let answered = match read {
            Some(DshStreamLine::Event) => self.drain(emit),
            // Every other reading — noise, an unreadable line, an idle
            // tick, the settle behind an exit, or no line at all — polls
            // the transcript. Enumerated so a new reading needs its own
            // disposition (#430 L3; decision 0071 ruling 2).
            Some(
                DshStreamLine::Malformed
                | DshStreamLine::Unreadable
                | DshStreamLine::Idle
                | DshStreamLine::End,
            )
            | None => self.poll(emit, observer),
        };
        // The answer is the whole report: stop here, not at EOF.
        if answered {
            return Ok(DshPass::Answered);
        }
        if !*open {
            if let Some(code) = wait(child)? {
                return Ok(DshPass::Exited(code));
            }
        }
        Ok(DshPass::Continue)
    }
}

/// One qualified stream-json invocation's inputs: the command it spawns,
/// the launch and bindings it serves, and the mutable state its passes fold
/// into. A parameter object, so the route takes one bundle rather than ten
/// positional arguments (decision 0071 rulings 4 and 10).
struct DshStreamCall<'a> {
    command: &'a [String],
    launch: &'a DshLaunch,
    bindings: &'a [secret::BoundSecret],
    probe: &'a mut ProbeAnswer,
    watch: &'a mut DshRootWatch,
    hold: &'a mut LaunchHold,
    session_meta: &'a mut Map<String, Value>,
}

/// The qualified `--output-format stream-json` exchange (task 8.8(d)):
/// stdout is the pinned plugin's two-event envelope, whose
/// post-`await agents.resume` init event is the ONLY root confirmation this
/// driver accepts. The retained transcript is folded alongside it, only
/// past the offered root's own sequence boundary, so a warm session never
/// re-counts its restored history.
///
/// stdout is read on its own thread, and the transcript is polled on EVERY
/// pass — a decoded line, noise, silence or EOF — so a probe reports its
/// answer at the answer rather than at stdout's end (fix round 430s4a3 M2;
/// decision 0103 ruling 4(a)).
fn invoke_dsh_stream_json(
    call: DshStreamCall<'_>,
    emit: &mut impl FnMut(&Value),
    observer: &mut impl FnMut(&DshObservation),
    mut wait: impl FnMut(&mut std::process::Child) -> std::io::Result<Option<i32>>,
) -> Result<DshRun, String> {
    let DshStreamCall {
        command,
        launch,
        bindings,
        probe,
        watch,
        hold,
        session_meta,
    } = call;
    let (mut child, stderr_thread) =
        spawn_dsh(command, &launch.workdir, Stdio::piped(), launch, bindings)?;
    let stdout = child.stdout.take().expect("piped");
    let mut turns = 0u64;
    let mut tail = DshTail::default();
    let (lines, reads) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        for line in std::io::BufReader::new(stdout).lines() {
            let failed = line.is_err();
            if lines.send(line).is_err() {
                return;
            }
            // A line the reader could not return ends stdout here, after
            // sending it: the thread returns, dropping the pipe, so a
            // child that keeps writing meets a closed stdout instead of
            // filling a channel nothing reads (#430 M1; decision 0103
            // ruling 4(a)).
            if failed {
                return;
            }
        }
    });
    let mut state = DshStreamState {
        launch,
        bindings,
        watch,
        hold,
        tail: &mut tail,
        turns: &mut turns,
        session_meta,
        probe,
        refusal: None,
    };
    // stdout may close before the child does (or stay open while it
    // sleeps): once it is closed, the transcript alone carries the answer
    // and the child's own exit ends the wait.
    let mut open = true;
    let mut stopped = false;
    // Every exit from this loop either answered or saw the child's status,
    // so the ONE status comes out of the loop and no wait after it is
    // unreachable.
    let status = loop {
        match io_context(
            state.pass(&mut open, &reads, &mut child, &mut wait, emit, observer),
            "agent CLI did not conclude",
        )? {
            DshPass::Continue => {}
            DshPass::Answered => {
                stopped = true;
                break lost::exit_code(conclude_harness(&mut child)?);
            }
            DshPass::Exited(code) => break code,
        }
    };
    // A clean end adds no confirmation fact and clears nothing.
    state.settle(DshStreamLine::End, emit, observer);
    if !stopped {
        state.drain(emit);
    }
    let captured = stderr_thread.take(stopped);
    finish_dsh(
        state.session_meta,
        status,
        captured,
        state.refusal.take(),
        state.probe.take(),
        bindings,
    )
}

/// Why a launch is refused before any row names its home or any child
/// spawns: the home's own refusal, the probe's argv refusal, or a composed
/// tree that does not hold (#896; decision 0071 ruling 8).
#[derive(Debug, thiserror::Error)]
enum PreSpawnRefusal {
    #[error(transparent)]
    Home(#[from] crate::transcript::DshHomeError),
    #[error(transparent)]
    Command(#[from] HarnessCommandRefusal),
    #[error(transparent)]
    Composition(#[from] dsh_hands::Composition),
}

/// The refusals a launch meets before any row names its home or any child
/// spawns: a binding over the home's fixed env, then, for a hands seat, a
/// composed tree that does not hold (#896).
fn refused_before_spawn(
    launch: &DshLaunch,
    workdir: &str,
    bindings: &[secret::BoundSecret],
) -> Result<(), PreSpawnRefusal> {
    let declared = bindings.iter().map(secret::BoundSecret::name);
    launch.home.serve(&dsh_child_env(launch), declared)?;
    if launch.mode.is_some() {
        dsh_hands::composition_gate(probe_command(launch, workdir, bindings)?)?;
    }
    Ok(())
}

/// The composition probe's argv: the session's own argv prefix with
/// `--dump-config` where the session carries its task, built by the ONE
/// spawn builder under the seat's own working directory, environment,
/// bindings and removals, so the probe and the session cannot disagree
/// about the context they run in (#896 round 3).
fn probe_command(
    launch: &DshLaunch,
    workdir: &str,
    bindings: &[secret::BoundSecret],
) -> Result<std::process::Command, HarnessCommandRefusal> {
    let mut command = launch.command.clone();
    command.push("--dump-config".to_string());
    harness_command(&command, workdir, &dsh_child_env(launch), bindings)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::adapters::tests::binding;
    use crate::env_guard::EnvGuard;
    use crate::transcript::{DshHome, DshHomeError};

    /// The hands launch a pre-spawn refusal is judged on: a driver the
    /// probe cannot run, atop `home`.
    fn hands_launch(dir: &std::path::Path, home: DshHome) -> DshLaunch {
        let root = dir.join("root");
        std::fs::create_dir_all(&root).unwrap();
        DshLaunch {
            command: vec!["/nonexistent/dsh".to_string()],
            rejoining: None,
            refusal: None,
            observed: Some("0.2.1-rc.2".to_string()),
            wrapper_digest: None,
            stream_json: false,
            first_seq: None,
            locator: "seat".to_string(),
            root: root.clone(),
            workdir: ".".to_string(),
            overlay: dsh_seat_overlay_with(None, None, &root, None, None).unwrap(),
            effortless: true,
            facts: crate::hands::GitFacts::default(),
            staged: None,
            home,
            mode: Some(dsh_hands::Mode::ReadOnly),
        }
    }

    /// A binding the injector refuses because its value is not UTF-8.
    fn invalid_utf8() -> Vec<secret::BoundSecret> {
        let store = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(store.path(), b"TOKEN=abcd\xff\n").unwrap();
        secret::resolve_bindings(store.path(), &["TOKEN".into()]).unwrap()
    }

    /// A hands seat whose driver cannot compose its tree is refused before
    /// any child spawns: the composed-tree check is on this launch's path,
    /// not only in a helper.
    #[test]
    fn a_hands_launch_is_refused_before_it_spawns_when_its_tree_is_unreadable() {
        let dir = tempfile::tempdir().unwrap();
        let launch = hands_launch(dir.path(), DshHome::operator().unwrap());
        // The refused launch never reaches its three observers, so each is
        // exercised once here and then handed to the invocation, whose
        // refusal proves they were not needed.
        let mut emit = |_: &Value| {};
        let wait = |_: &mut std::process::Child| Ok(None);
        let mut observer = |_: &DshObservation| {};
        emit(&Value::Null);
        let mut child = std::process::Command::new("true").spawn().unwrap();
        assert_eq!(wait(&mut child).unwrap(), None);
        let _ = child.wait();
        observer(&DshObservation {
            line: DshStreamLine::End,
            census: None,
            last_seq: None,
            disposition: DshDisposition::Cold,
        });
        let problem = invoke_dsh_launch_observed(
            launch,
            vec!["/nonexistent/dsh".to_string()],
            &[],
            &mut ProbeAnswer::new(false),
            &mut emit,
            wait,
            &mut observer,
        )
        .err()
        .expect("the unreadable composed tree refuses");
        assert_eq!(problem, dsh_hands::Composition::Unreadable.to_string());
    }

    /// Each pre-spawn refusal is typed by the error it carries, matched by
    /// variant, and prints today's full words (#896 round 6; decision 0071
    /// ruling 8).
    #[test]
    fn the_pre_spawn_refusals_are_typed_and_keep_their_words() {
        let mut env = EnvGuard::lock();
        let dir = tempfile::tempdir().unwrap();
        let workdir = dir.path().to_str().unwrap();
        let unreadable = refused_before_spawn(
            &hands_launch(dir.path(), DshHome::operator().unwrap()),
            workdir,
            &[],
        )
        .unwrap_err();
        assert!(matches!(
            unreadable,
            PreSpawnRefusal::Composition(dsh_hands::Composition::Unreadable)
        ));
        assert_eq!(
            unreadable.to_string(),
            dsh_hands::Composition::Unreadable.to_string()
        );

        env.set("HOME", dir.path());
        let engine = DshHome::stage(workdir).unwrap();
        let fixed = refused_before_spawn(
            &hands_launch(dir.path(), engine),
            workdir,
            &[binding("DSH_HOME", "elsewhere")],
        )
        .unwrap_err();
        // The variant here, the name in its words below: a guard on the
        // name would be a branch no refusal takes.
        assert!(matches!(
            fixed,
            PreSpawnRefusal::Home(DshHomeError::FixedBinding(_))
        ));
        assert_eq!(
            fixed.to_string(),
            DshHomeError::FixedBinding("DSH_HOME".into()).to_string()
        );

        let invalid = invalid_utf8();
        let refused = refused_before_spawn(
            &hands_launch(dir.path(), DshHome::operator().unwrap()),
            workdir,
            &invalid,
        )
        .unwrap_err();
        assert!(matches!(
            refused,
            PreSpawnRefusal::Command(HarnessCommandRefusal::Secret(secret::BindError::NotUtf8(_)))
        ));
        assert_eq!(refused.to_string(), "secret 'TOKEN' is not valid UTF-8");
    }

    /// The between-lines poll is driven by the watch's three conditions:
    /// a cold seat has no offer to confirm, a rejoin whose init event has
    /// not named the offer has nothing to settle, a settled watch is not
    /// revisited, and only a named, unsettled rejoin settles on the idle
    /// tick (decision 0103 ruling 4(a); design D7).
    #[test]
    fn the_between_lines_poll_settles_only_a_named_unsettled_offer() {
        fn poll_once(launch: &DshLaunch, watch: &mut DshRootWatch) -> (bool, usize) {
            let mut hold = LaunchHold::new("deepseek", launch.plan());
            let mut tail = DshTail::default();
            let mut turns = 0u64;
            let mut meta = Map::new();
            let mut probe = ProbeAnswer::new(true);
            let mut observed = 0usize;
            let mut state = DshStreamState {
                launch,
                bindings: &[],
                watch,
                hold: &mut hold,
                tail: &mut tail,
                turns: &mut turns,
                session_meta: &mut meta,
                probe: &mut probe,
                refusal: None,
            };
            let answered = state.poll(&mut |_| {}, &mut |_: &DshObservation| {
                observed += 1;
            });
            (answered, observed)
        }

        let dir = tempfile::tempdir().unwrap();
        crate::adapters::tests::plant_dsh_session(dir.path(), "seat", "--w--", "session-1", 27);
        let root = dir.path().join("seat");
        let shim =
            crate::adapters::tests::executable(dir.path(), "dsh-poll", "#!/bin/sh\nexit 0\n");

        // A cold launch never enters the confirmation poll.
        let cold = crate::adapters::tests::dsh_stream_launch(&shim, &root, None, None);
        let mut cold_watch = DshRootWatch::new(&cold, Transcript::dsh(&cold.home));
        assert_eq!(poll_once(&cold, &mut cold_watch), (false, 0));

        // A named, unsettled rejoin settles on the idle tick.
        let launch =
            crate::adapters::tests::dsh_stream_launch(&shim, &root, Some("session-1"), Some(27));
        let mut pending = DshRootWatch::new(&launch, Transcript::dsh(&launch.home));
        assert!(pending.confirming());
        assert!(!pending.settled);
        pending.named_the_offer = true;
        assert_eq!(poll_once(&launch, &mut pending), (false, 1));

        // An offer the init event has not named has nothing to settle.
        let mut unnamed = DshRootWatch::new(&launch, Transcript::dsh(&launch.home));
        unnamed.named_the_offer = false;
        assert_eq!(poll_once(&launch, &mut unnamed), (false, 0));

        // A settled watch is terminal: no later idle tick revisits it.
        let mut settled = DshRootWatch::new(&launch, Transcript::dsh(&launch.home));
        settled.named_the_offer = true;
        settled.refuse();
        assert_eq!(poll_once(&launch, &mut settled), (false, 0));
    }

    /// The one harness spawn builder's refusals are typed, carry the
    /// injector's own error, and print exactly the words both existing text
    /// boundaries printed before (#896 round 4; decision 0071 ruling 8).
    #[test]
    fn harness_command_refusals_are_typed_and_keep_their_words() {
        let empty = harness_command(&[], "", &[], &[]).unwrap_err();
        assert_eq!(empty, HarnessCommandRefusal::Empty);
        assert_eq!(empty.to_string(), "empty command");

        let invalid = invalid_utf8();
        let refused = harness_command(&["true".into()], "", &[], &invalid).unwrap_err();
        assert_eq!(
            refused,
            HarnessCommandRefusal::Secret(secret::BindError::NotUtf8("TOKEN".into()))
        );
        assert_eq!(refused.to_string(), "secret 'TOKEN' is not valid UTF-8");
    }
}
