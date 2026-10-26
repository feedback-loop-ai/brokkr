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
    // Before any row names the home, a binding over its fixed env refuses.
    let declared = bindings.iter().map(secret::BoundSecret::name);
    launch.home.serve(&dsh_child_env(&launch), declared)?;
    if launch.hands {
        let bin = launch.command.first().map_or("", String::as_str);
        dsh_hands::composition_gate(bin, launch.overlay.path(), &dsh_child_env(&launch))?;
    }
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
    let mut invocation = match attempt {
        Ok(invocation) => invocation,
        Err(problem) => return Err(dsh_failure_before_promotion(problem, staged)),
    };
    if launch.hands {
        if let Err(problem) =
            dsh_hands::startup_gate(&invocation.stderr, &mut invocation.session_meta)
        {
            return Err(dsh_failure_before_promotion(problem, staged));
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

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    /// A hands seat whose driver cannot compose its tree is refused before
    /// any child spawns: the composed-tree check is on this launch's path,
    /// not only in a helper.
    #[test]
    fn a_hands_launch_is_refused_before_it_spawns_when_its_tree_is_unreadable() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root");
        std::fs::create_dir(&root).unwrap();
        let launch = DshLaunch {
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
            home: crate::transcript::DshHome::operator().unwrap(),
            hands: true,
        };
        let problem = invoke_dsh_launch_observed(
            launch,
            vec!["/nonexistent/dsh".to_string()],
            dir.path().to_str().unwrap(),
            &[],
            &mut |_| {},
            |_| Ok(None),
            &mut |_| {},
        )
        .err()
        .expect("the unreadable composed tree refuses");
        assert!(problem.contains("composed dsh tree"), "{problem}");
    }
}
