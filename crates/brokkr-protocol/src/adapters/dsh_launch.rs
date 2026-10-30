//! The one dsh invocation's lifecycle (#896): its spawn, fold, startup
//! report and commit promotion, moved out of `adapters.rs` when the
//! composed-tree and activation checks grew it past its measured ceiling.

use super::*;

/// `invoke_dsh_launch` with this ONE invocation's observer of the
/// confirmation's completed observations, spawning exactly `command`.
/// Production watches nothing; a test reads here exactly what the watcher
/// consumed — never a second walk of its own — and only once the watcher
/// has ruled on it.
pub(super) fn invoke_dsh_launch_observed(
    mut launch: DshLaunch,
    command: Vec<String>,
    workdir: &str,
    bindings: &[secret::BoundSecret],
    emit: &mut impl FnMut(&Value),
    wait: impl FnMut(&mut std::process::Child) -> std::io::Result<Option<i32>>,
    observer: &mut impl FnMut(&DshObservation),
) -> Result<Invocation, String> {
    refused_before_spawn(&launch, workdir, bindings).map_err(|refusal| refusal.to_string())?;
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
            &command,
            &launch,
            workdir,
            bindings,
            &mut watch,
            &mut hold,
            &mut session_meta,
            emit,
            observer,
        )
    } else {
        invoke_dsh_shipped(
            &command,
            workdir,
            &launch,
            bindings,
            wait,
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
            dir.path().to_str().unwrap(),
            &[],
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
