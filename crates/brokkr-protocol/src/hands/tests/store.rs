//! The secrets store hidden in every workspace box (decision 0065 slice
//! two, U8h; SI1, MB4), in real namespaces: a store the worktree, a bind or
//! the bundle root holds reads as denied to a workspace call and an exec
//! while the host's store is unchanged, its folders cannot be moved, a
//! store outside every mount is not mounted, and a store absent in reach,
//! with a symbolic link on its sealed path, of two links, open to its group
//! or others, shown in reach by another mount, under an overlay, holding
//! the Git directory in its folders or replaced between the box's two
//! observations refuses with the one cause before the command runs. A store renamed over while one box runs is
//! shown to do what operator ruling 3 of 2026-10-23 records.

use super::super::evidence::CONFIRMING;
use super::super::sealed::{Unhidden, TABLE};
use super::*;

/// The fake value every planted store holds; no real store is read.
const FAKE: &str = "DOCS_TOKEN=fake-u8h-value\n";

/// A worktree under a canonical temporary root, a store sealed at `store`
/// in the reach of `spec`'s boxes, its session and an artifact root
/// outside every mount.
struct Stored {
    dir: tempfile::TempDir,
    root: PathBuf,
    work: PathBuf,
    store: PathBuf,
    spec: HandsSpec,
    reach: Sealed,
    session: Session,
    artifacts: PathBuf,
}

impl Stored {
    /// A store at `store` below the root holding [`FAKE`], owner-only,
    /// `binds` bound as authored below the root, each `(path, mode)`; none
    /// where this host cannot make a namespace.
    fn at(store: &str, binds: &[(&str, &str)]) -> Option<Stored> {
        let mut stored = Stored::sealed(store, binds)?;
        std::fs::create_dir_all(stored.store.parent().unwrap()).unwrap();
        planted(&stored.store, FAKE);
        stored.reseal(stored.store.clone());
        Some(stored)
    }

    /// [`Stored::at`] with nothing planted at `store`.
    fn sealed(store: &str, binds: &[(&str, &str)]) -> Option<Stored> {
        if !can_create_namespace() {
            let reason = "this environment cannot create a bubblewrap namespace";
            skip_boundary_proof(boundary_evidence_required(), reason);
            return None;
        }
        let (dir, root, work) = fixture();
        let binds: Vec<(PathBuf, &str)> = (binds.iter())
            .map(|(path, mode)| (root.join(path), *mode))
            .collect();
        for (path, _) in &binds {
            std::fs::create_dir_all(path).unwrap();
        }
        let binds: Vec<(&Path, &str)> = binds.iter().map(|(p, m)| (p.as_path(), *m)).collect();
        let spec = spec_of(json!({"kind": "workspace", "binds": binds
            .iter()
            .map(|(path, mode)| json!({"path": path, "mode": mode}))
            .collect::<Vec<_>>()}));
        let session = Session::create_under(dir.path(), "test").unwrap();
        let artifacts = artifact_root(&root.join("run"));
        let mut stored = Stored {
            dir,
            store: root.join(store),
            root,
            work,
            spec,
            reach: Sealed::default(),
            session,
            artifacts,
        };
        stored.reseal(stored.store.clone());
        Some(stored)
    }

    /// Seal the reach again, its store now configured at `store`, at the
    /// place [`store_place`] resolves it to, as the engine keeps it; one no
    /// walk resolves is sealed as spelled, as no engine seals it, so the box
    /// is shown judging such a reach on its own.
    fn reseal(&mut self, store: PathBuf) {
        let (home, git) = (self.root.join("home"), git_facts(&self.work));
        let place = store_place(&store).unwrap_or_else(|_| store.clone());
        self.reach = seal(&self.spec, &home, &self.work, &git, Some((&place, &store)));
        self.store = store;
    }

    /// The next workspace call, as `hands serve` runs it, running
    /// `command` in its box.
    fn call(&self, command: &str) -> Result<Executed, String> {
        let at = self.session.path();
        let long = Duration::from_secs(30);
        let (spec, work, reach) = (&self.spec, &self.work, &self.reach);
        execute(spec, work, &self.artifacts, reach, at, command, long)
    }

    /// The next exec, as `hands exec` boxes it whole, running `command`.
    fn exec(&self, command: &str) -> Result<i32, String> {
        self.exec_in(command, None)
    }

    /// [`Stored::exec`] handed the bundle root `bundle`.
    fn exec_in(&self, command: &str, bundle: Option<&Path>) -> Result<i32, String> {
        let bwrap = require_bwrap().unwrap();
        let session = self
            .dir
            .path()
            .join(format!("exec-{}", uuid::Uuid::new_v4()));
        let run = ["/bin/sh", "-c", command].map(String::from);
        let (spec, work, reach) = (&self.spec, &self.work, &self.reach);
        let artifacts = &self.artifacts;
        run_boxed_in(
            &bwrap,
            spec,
            work,
            artifacts,
            reach,
            &session,
            &[],
            bundle,
            &run,
        )
    }

    /// The argv of the next box, or why the store is not hidden in it.
    fn built(&self) -> Result<Vec<String>, Unhidden> {
        let dir = self
            .dir
            .path()
            .join(format!("built-{}", uuid::Uuid::new_v4()));
        let built = boxed_from(&self.spec, &self.work, &self.artifacts, &self.reach, &dir);
        built.map_err(|error| *error.into_inner().unwrap().downcast::<Unhidden>().unwrap())
    }

    /// The refusal `unhidden`, as a workspace call and an exec return it.
    fn refused(&self, unhidden: &Unhidden) {
        let text = format!("hands namespace: {unhidden}");
        let marker = self.work.join("ran");
        let called = self.call("touch ran").map(|ran| ran.exit_code);
        assert_eq!(called, Err(text.clone()));
        assert_eq!(self.exec("touch ran"), Err(text));
        assert!(!marker.exists(), "the command never ran");
    }

    /// A workspace call's `mv from to`, refused as a mount point cannot be
    /// renamed (U0b X31).
    fn busy(&self, from: &str, to: &str) {
        let ran = self.call(&format!("mv {from} {to}")).unwrap();
        let text = format!("mv: cannot move '{from}' to '{to}': Device or resource busy\n");
        assert_eq!((ran.stderr, ran.exit_code), (text, 1), "{from}");
    }

    /// Ruling 10's exact denial of the store at `path`, through a workspace
    /// call's `cat` and an exec's, each its stderr and exit code; and the
    /// host's store still [`FAKE`].
    fn denies(&self, path: &Path) {
        let ran = self.call(&format!("cat {}", path.display())).unwrap();
        assert_eq!((ran.stdout.as_str(), ran.exit_code), ("", 1), "the call");
        denied(path, &ran.stderr);
        self.exec_denies(path, None);
        assert_eq!(std::fs::read_to_string(&self.store).unwrap(), FAKE);
    }

    /// Ruling 10's exact denial of the store at `path` to an exec handed
    /// the bundle root `bundle`.
    fn exec_denies(&self, path: &Path, bundle: Option<&Path>) {
        let exec = format!("exec 2>seen; exec cat {}", path.display());
        assert_eq!(self.exec_in(&exec, bundle), Ok(1), "the exec");
        denied(
            path,
            &std::fs::read_to_string(self.work.join("seen")).unwrap(),
        );
    }
}

/// Plant `text` at `path`, owner-only, as the engine makes a store.
fn planted(path: &Path, text: &str) {
    std::fs::write(path, text).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
}

/// Assert `stderr` is what `cat` says reading a hidden store at `path`:
/// `/dev/null`'s device, bound `nodev` as every bubblewrap bind is, cannot
/// be opened (EACCES), in GNU coreutils' words or in uutils', which name
/// the error number. The box runs the host's own `cat`.
fn denied(path: &Path, stderr: &str) {
    let words = format!("cat: {}: Permission denied", path.display());
    let either = [format!("{words}\n"), format!("{words} (os error 13)\n")];
    assert!(either.iter().any(|denial| denial == stderr), "{stderr:?}");
}

/// A store the worktree holds, sealed at its own spelling or through a
/// link a system folder holds as it is sealed, which the reach resolves
/// then to the store's canonical path (ruling 15), reads as denied to a
/// workspace call and an exec, and the host's store is unchanged. Its
/// folder is pinned after the worktree's own bind, and the hide comes after
/// both.
#[test]
fn a_store_in_the_worktree_is_denied_to_a_workspace_call_or_an_exec() {
    for spelled in ["work/.forge/secrets.env", "outside/link/secrets.env"] {
        let Some((mut stored, held, _)) = linked_outside("link") else {
            return;
        };
        stored.reseal(stored.root.join(spelled));
        let sealed = serde_json::to_value(&stored.reach).unwrap()["store"].clone();
        let both = json!({"place": held, "spelling": stored.root.join(spelled)});
        assert_eq!(
            sealed, both,
            "the reach seals the canonical path beside its spelling"
        );
        stored.denies(&held);
        let argv = stored.built().unwrap().join(" ");
        let worked = position(&argv, pin(&stored.work));
        let pinned = position(&argv, pin(&stored.work.join(".forge")));
        let hidden = position(&argv, hide(&held));
        assert!((worked < pinned) & (pinned < hidden), "{argv}");
    }
}

/// A store planted in the worktree's `.forge`, its place, and a link
/// `outside/<name>` to that folder, which a system folder outside every
/// mount holds; none where this host cannot make a namespace.
fn linked_outside(name: &str) -> Option<(Stored, PathBuf, PathBuf)> {
    let stored = Stored::at("work/.forge/secrets.env", &[])?;
    let held = stored.work.join(".forge/secrets.env");
    std::fs::create_dir(stored.root.join("outside")).unwrap();
    let link = stored.root.join("outside").join(name);
    std::os::unix::fs::symlink("../work/.forge", &link).unwrap();
    Some((stored, held, link))
}

/// Replace the folder or file at `at` on the host with a symbolic link to
/// where it now lies, `at` with `-real` appended, as a call or a user could
/// once the reach is sealed.
fn relinked(at: &Path) {
    let mut real = at.as_os_str().to_owned();
    real.push("-real");
    std::fs::rename(at, &real).unwrap();
    std::os::unix::fs::symlink(&real, at).unwrap();
}

/// Undo [`relinked`] at `at`.
fn unlinked(at: &Path) {
    let mut real = at.as_os_str().to_owned();
    real.push("-real");
    std::fs::remove_file(at).unwrap();
    std::fs::rename(&real, at).unwrap();
}

/// Operator ruling 15 of 2026-10-24: once the reach seals the store's
/// canonical path, a symbolic link that comes to lie anywhere on it, at a
/// folder the worktree holds, at the final entry, at a folder outside every
/// mount or in a folder the box shows twice, through the worktree and
/// through a bind spelled by another name, refuses in every later box
/// before the command runs, whatever mount holds it, and the store is
/// unchanged. A store `brokkr secrets set` renames over adds no link, and
/// the next box hides it (ruling 5).
#[test]
fn a_link_on_the_sealed_store_path_refuses_wherever_it_lies() {
    let Some(stored) = Stored::at("work/conf/secrets.env", &[]) else {
        return;
    };
    let conf = stored.work.join("conf");
    relinked(&conf);
    let routed = Unhidden::Routed(stored.store.clone(), conf.clone());
    stored.refused(&routed);
    let text = format!(
        "secrets store cannot be hidden from workspace hands: the symbolic link '{}' lies on the \
         sealed path '{}', so it no longer resolves to itself",
        conf.display(),
        stored.store.display()
    );
    assert_eq!(routed.to_string(), text);
    unlinked(&conf);
    relinked(&stored.store);
    stored.refused(&Unhidden::Routed(
        stored.store.clone(),
        stored.store.clone(),
    ));
    unlinked(&stored.store);
    let new = stored.work.join("conf/.secrets.env.new");
    planted(&new, FAKE);
    std::fs::rename(&new, &stored.store).unwrap();
    stored.denies(&stored.store);
    let Some(outside) = Stored::at("outside/sys/secrets.env", &[]) else {
        return;
    };
    let sys = outside.root.join("outside/sys");
    relinked(&sys);
    outside.refused(&Unhidden::Routed(outside.store.clone(), sys));
    let Some(mut twice) = Stored::at("work/shared/deep/secrets.env", &[]) else {
        return;
    };
    let tools = twice.root.join("tools");
    std::os::unix::fs::symlink(twice.work.join("shared"), &tools).unwrap();
    let bound = json!({"kind": "workspace", "binds": [{"path": tools, "mode": "rw"}]});
    twice.spec = spec_of(bound);
    twice.reseal(twice.store.clone());
    twice.denies(&twice.store);
    twice.exec_denies(&tools.join("deep/secrets.env"), None);
    let deep = twice.work.join("shared/deep");
    relinked(&deep);
    twice.refused(&Unhidden::Routed(twice.store.clone(), deep));
    assert_eq!(std::fs::read_to_string(&twice.store).unwrap(), FAKE);
}

/// Operator ruling 17 of 2026-10-25: the reach seals the store's place
/// beside the spelling the engine configured it at, here through a link a
/// system folder holds. An update `brokkr secrets set` renames over at that
/// unchanged spelling is hidden in the next box; once the link moves, the
/// spelling leads where no box would hide the store, and every later box
/// refuses before the command runs, the store at both places unchanged; so
/// does every box once the spelling no longer resolves at all (ruling 18).
#[test]
fn a_configured_spelling_that_no_longer_leads_to_the_sealed_store_refuses() {
    let Some((mut stored, held, route)) = linked_outside("route") else {
        return;
    };
    stored.reseal(route.join("secrets.env"));
    let new = stored.work.join(".forge/.secrets.env.new");
    planted(&new, FAKE);
    std::fs::rename(&new, &held).unwrap();
    stored.denies(&held);
    let moved = stored.work.join("moved");
    std::fs::create_dir(&moved).unwrap();
    planted(&moved.join("secrets.env"), FAKE);
    std::fs::remove_file(&route).unwrap();
    std::os::unix::fs::symlink(&moved, &route).unwrap();
    let strayed = Unhidden::Strayed(route.join("secrets.env"), held.clone());
    stored.refused(&strayed);
    let text = format!(
        "secrets store cannot be hidden from workspace hands: the configured store '{}' no \
         longer leads to its sealed place '{}'",
        route.join("secrets.env").display(),
        held.display()
    );
    assert_eq!(strayed.to_string(), text);
    let file = stored.root.join("outside/file");
    std::fs::write(&file, "").unwrap();
    std::fs::remove_file(&route).unwrap();
    std::os::unix::fs::symlink(&file, &route).unwrap();
    let kind = std::io::ErrorKind::NotADirectory;
    stored.refused(&Unhidden::Unobserved(route.join("secrets.env"), kind));
    let both = [&held, &moved.join("secrets.env")].map(|at| std::fs::read_to_string(at).unwrap());
    assert_eq!(both, [FAKE, FAKE]);
}

/// A store a bind holds outside the worktree is hidden where the bind
/// shows it, after the bind's own mount; a store outside every mount is
/// never mounted, and the box does not see it at all.
#[test]
fn a_store_a_bind_holds_is_hidden_and_one_outside_every_mount_is_not_mounted() {
    let Some(stored) = Stored::at("tools/secrets.env", &[("tools", "ro")]) else {
        return;
    };
    stored.denies(&stored.store);
    let argv = stored.built().unwrap().join(" ");
    let tools = stored.root.join("tools");
    let bind = format!("--ro-bind-try {t} {t}", t = tools.display());
    let hide = format!("--ro-bind /dev/null {}", stored.store.display());
    assert!(argv.find(&bind) < argv.find(&hide), "{argv}");
    let Some(outside) = Stored::at("outside/secrets.env", &[]) else {
        return;
    };
    let argv = outside.built().unwrap().join(" ");
    assert!(!argv.contains("/dev/null"), "{argv}");
    assert!(!argv.contains("outside"), "{argv}");
    let probe = format!(
        "test -e {} && echo shown; echo done",
        outside.store.display()
    );
    assert_eq!(outside.call(&probe).unwrap().stdout, "done\n");
}

/// Operator ruling 6 of 2026-10-24 (U0b X31): every folder between a
/// writable mount's root and the store is a mount of its own, shallowest
/// first, so a call cannot move the store's folder and leave a decoy at
/// its old spelling; the next box still hides the store. Shown for a store
/// two folders deep in the worktree, which no artifact root pins, and for
/// one in a read-write bind, pinned after that bind's own mount.
#[test]
fn a_call_cannot_move_the_store_s_folders_and_the_next_box_still_hides_it() {
    let Some(stored) = Stored::at("work/conf/deep/secrets.env", &[]) else {
        return;
    };
    stored.busy("conf", "conf-moved");
    stored.busy("conf/deep", "conf/moved");
    assert!(stored.store.exists() & !stored.work.join("conf-moved").exists());
    stored.denies(&stored.store);
    let argv = stored.built().unwrap().join(" ");
    let (conf, deep) = (stored.work.join("conf"), stored.work.join("conf/deep"));
    let first = position(&argv, pin(&conf));
    let second = position(&argv, pin(&deep));
    let hidden = position(&argv, hide(&stored.store));
    assert!((first < second) & (second < hidden), "{argv}");
    let Some(bound) = Stored::at("tools/conf/secrets.env", &[("tools", "rw")]) else {
        return;
    };
    let (tools, conf) = (bound.root.join("tools"), bound.root.join("tools/conf"));
    let moved = tools.join("moved");
    bound.busy(&conf.display().to_string(), &moved.display().to_string());
    bound.denies(&bound.store);
    let argv = bound.built().unwrap().join(" ");
    let mounted = position(&argv, format!("--bind-try {t} {t}", t = tools.display()));
    assert!(mounted < position(&argv, pin(&conf)), "{argv}");
}

/// Operator ruling 13 of 2026-10-24: the folders the worktree holds above
/// the store and above the artifact root are pinned together, outermost
/// first and each once, so a shared shallow folder pinned again never
/// shadows the other chain's deeper pins: no call moves either chain.
#[test]
fn the_store_s_and_the_artifact_root_s_folders_are_pinned_together() {
    let Some(mut stored) = Stored::at("work/s/keys/secrets.env", &[]) else {
        return;
    };
    stored.artifacts = artifact_root(&stored.work.join("s/run"));
    stored.busy("s/run", "s/run-moved");
    stored.busy("s/keys", "s/keys-moved");
    stored.busy("s", "s-moved");
    stored.denies(&stored.store);
    let table = stored.call("cat /proc/self/mountinfo").unwrap().stdout;
    let at = |point: &Path| -> Vec<(&str, &str)> {
        let point = point.display().to_string();
        let lines = table
            .lines()
            .map(|line| line.split(' ').collect::<Vec<_>>());
        let mut shown = lines.filter(|fields| fields[4] == point);
        let ids = shown.by_ref().map(|fields| (fields[0], fields[1]));
        ids.collect()
    };
    let shared = at(&stored.work.join("s"));
    let top = shared.last().map(|(id, _)| *id);
    for deeper in ["s/keys", "s/run"] {
        let parents: Vec<&str> = at(&stored.work.join(deeper))
            .iter()
            .map(|(_, up)| *up)
            .collect();
        assert_eq!(
            parents.last().copied(),
            top,
            "{deeper} is pinned on the topmost shared pin: {table}"
        );
    }
    assert_eq!(shared.len(), 1, "the shared folder is pinned once: {table}");
}

/// Ruling 13 over the one description of the box's mounts: a hide is
/// judged by the last mount over its place, a pin the box generates as
/// much as a declared bind, so a pin over the store's folder that shows
/// another folder there refuses the hide planned beneath it.
#[test]
fn a_generated_pin_that_would_shadow_a_hide_refuses() {
    use super::super::sealed::{placed, Held, Mounted};
    let mounted = |host: &str, shown: &str, held| Mounted {
        host: PathBuf::from(host),
        shown: PathBuf::from(shown),
        held,
        bind: None,
    };
    let (place, hidden) = (Path::new("/h/s/secrets.env"), "/w/s/secrets.env");
    let worktree = mounted("/h", "/w", Held::Writable);
    let own = mounted("/h/s", "/w/s", Held::Pinned);
    let held = [worktree.clone(), own];
    assert_eq!(placed(place, &held), Ok(vec![hidden.to_owned()]));
    let other = mounted("/elsewhere/s", "/w/s", Held::Pinned);
    let shadowed = Unhidden::Shadowed(place.to_path_buf(), PathBuf::from(hidden));
    assert_eq!(placed(place, &[worktree, other]), Err(shadowed));
}

/// Run 4's L1 and run 5's L2: the one description of every place a box
/// mounts, which the store's pins and hides are planned from, is the box's
/// own, and nothing else the box mounts can show the store. Every
/// filesystem step `box_argv` emits, in order and by its exact flag, is the
/// one this test names: `/proc`, `/dev` and the box's folders; the
/// toolchain; the identity files, home and tmp under the box's scratch; the
/// worktree, the bundle root and a linked worktree's Git common directory,
/// its hooks hidden behind a tmpfs and its config read-only; the artifact
/// root's pinned folders; a read-only bind and its mask; a writable bind
/// and the store's folder pinned after it; an overlay; the store's hide;
/// and the artifact root, read-only last. The description holds exactly
/// the steps marked with how it holds them, in that order.
#[test]
fn every_mount_the_box_makes_is_the_store_s_description_in_order() {
    use super::super::evidence::Protected;
    use super::super::sealed::{Held, Hidden};
    let binds = [("tools", "ro"), ("cache", "rw"), ("layer", "overlay")];
    let Some(mut stored) = Stored::sealed("cache/deep/secrets.env", &binds) else {
        return;
    };
    let main = stored.root.join("main");
    std::fs::create_dir(&main).unwrap();
    linked_worktree(&main, &stored.work);
    std::fs::create_dir(stored.root.join("cache/deep")).unwrap();
    planted(&stored.store, FAKE);
    std::fs::write(stored.root.join("tools/masked"), "").unwrap();
    stored.spec.binds[0].mask = vec!["masked".into()];
    stored.artifacts = artifact_root(&stored.work.join("s/run"));
    stored.reseal(stored.store.clone());
    let (bundle, dir) = (stored.root.join("bundle"), stored.dir.path().join("parity"));
    std::fs::create_dir(&bundle).unwrap();
    let (scratch, session) = (dir.join("scratch"), dir.join("session"));
    let (spec, work, reach) = (&stored.spec, &stored.work, &stored.reach);
    let writes = OverlayWrites::Session(&session);
    let made = box_argv(
        spec,
        work,
        &stored.artifacts,
        reach,
        &scratch,
        writes,
        &[],
        Some(&bundle),
        &one("true"),
    );
    let argv = made.unwrap();
    let common = main.join(".git");
    let planned = planned(&stored, (&scratch, &session), (&bundle, &common));
    let steps: Vec<&[String]> = planned.iter().map(|(step, _)| step.as_slice()).collect();
    assert_eq!(emitted(&argv), steps, "{argv:?}");
    let protected = Protected::observe(&stored.artifacts, work).unwrap();
    let artifacts = (protected.held(), protected.ancestors());
    let hidden = Hidden::observe(spec, reach, (work, Some(&bundle)), artifacts).unwrap();
    let canonical = |at: &Path| std::fs::canonicalize(at).unwrap_or(at.to_path_buf());
    let described: Vec<(Held, PathBuf, String)> = (hidden.mounts().iter())
        .map(|mount| (mount.held, mount.host.clone(), namespace_path(&mount.shown)))
        .collect();
    let held = planned
        .iter()
        .filter_map(|(step, held)| Some(((*held)?, step)));
    let named = held.map(|(held, step)| {
        let source = Path::new(&step[step.len() - 2]);
        let source = if held == Held::Overlay {
            Path::new(&step[1])
        } else {
            source
        };
        (held, canonical(source), step[step.len() - 1].clone())
    });
    assert_eq!(described, named.collect::<Vec<_>>());
}

/// Every filesystem step the box of `stored` is to make, in order, each
/// beside how the store's description holds it, where it does: its scratch
/// and its overlay `session`, its `bundle` root and the Git `common`
/// directory as the parity test above builds them.
fn planned(
    stored: &Stored,
    (scratch, session): (&Path, &Path),
    (bundle, common): (&Path, &Path),
) -> Vec<(Vec<String>, Option<super::super::sealed::Held>)> {
    use super::super::sealed::Held;
    let (root, work) = (&stored.root, &stored.work);
    let at = |path: &Path| path.display().to_string();
    let step = |parts: &[&str]| {
        parts
            .iter()
            .map(|part| part.to_string())
            .collect::<Vec<_>>()
    };
    let mut planned = vec![
        (step(&["--proc", "/proc"]), None),
        (step(&["--dev", "/dev"]), None),
    ];
    for folder in ["/runtime", "/etc", "/home", "/root", "/run", "/usr"] {
        planned.push((step(&["--dir", folder]), None));
    }
    for &tool in HOST_TOOLCHAIN_BINDS {
        planned.push((step(&["--ro-bind-try", tool, tool]), Some(Held::ReadOnly)));
    }
    for file in ["passwd", "group", "hosts", "nsswitch.conf"] {
        let (source, shown) = (at(&scratch.join("etc").join(file)), format!("/etc/{file}"));
        planned.push((step(&["--ro-bind", &source, &shown]), None));
    }
    let (home, tmp) = (at(&scratch.join("home")), at(&scratch.join("tmp")));
    planned.push((step(&["--bind", &home, "/runtime/home"]), None));
    planned.push((step(&["--bind", &tmp, "/tmp"]), None));
    let both = |flag: &str, path: &Path| step(&[flag, &at(path), &at(path)]);
    planned.push((both("--bind", work), Some(Held::Writable)));
    let bundled = step(&["--ro-bind", &at(bundle), "/runtime/bundle"]);
    planned.push((bundled, Some(Held::ReadOnly)));
    planned.push((both("--bind", common), Some(Held::Writable)));
    planned.push((step(&["--tmpfs", &at(&common.join("hooks"))]), None));
    planned.push((both("--ro-bind-try", &common.join("config")), None));
    planned.push((both("--bind", &work.join("s")), Some(Held::Pinned)));
    planned.push((both("--bind", &work.join("s/run")), Some(Held::Pinned)));
    planned.push((
        both("--ro-bind-try", &root.join("tools")),
        Some(Held::ReadOnly),
    ));
    let masked = at(&root.join("tools/masked"));
    planned.push((step(&["--ro-bind", "/dev/null", &masked]), None));
    planned.push((
        both("--bind-try", &root.join("cache")),
        Some(Held::Writable),
    ));
    planned.push((both("--bind", &root.join("cache/deep")), Some(Held::Pinned)));
    let (layer, layers) = (at(&root.join("layer")), session.join("overlay/2"));
    let (upper, lower) = (at(&layers.join("upper")), at(&layers.join("work")));
    let overlaid = step(&["--overlay-src", &layer, "--overlay", &upper, &lower, &layer]);
    planned.push((overlaid, Some(Held::Overlay)));
    let hide = step(&["--ro-bind", "/dev/null", &at(&stored.store)]);
    planned.push((hide, None));
    planned.push((both("--ro-bind", &stored.artifacts), None));
    planned
}

/// Every filesystem step in a box's `argv` before the command, in order,
/// each its flag and the arguments it takes: an overlay's source with the
/// layers and destination that follow it.
fn emitted(argv: &[String]) -> Vec<&[String]> {
    let end = argv.iter().position(|part| part == "--").unwrap();
    let (mut steps, mut at) = (Vec::new(), 0);
    while at < end {
        let taken = match argv[at].as_str() {
            "--proc" | "--dev" | "--dir" | "--tmpfs" => 2,
            "--bind" | "--bind-try" | "--ro-bind" | "--ro-bind-try" => 3,
            "--overlay-src" if argv[at + 2] == "--overlay" => 6,
            "--overlay-src" => 4,
            _ => {
                at += 1;
                continue;
            }
        };
        steps.push(&argv[at..at + taken]);
        at += taken;
    }
    steps
}

/// Where `part` begins in the box's `argv`.
fn position(argv: &str, part: String) -> usize {
    argv.find(&part).unwrap_or_else(|| panic!("{part}: {argv}"))
}

/// The argv's pin of the folder `at` onto itself.
fn pin(at: &Path) -> String {
    format!("--bind {a} {a}", a = at.display())
}

/// The argv's hide of the store at `at`.
fn hide(at: &Path) -> String {
    format!("--ro-bind /dev/null {}", at.display())
}

/// Operator ruling 7 of 2026-10-24: the bundle root the box mounts at
/// `/runtime/bundle` is accounted for by source and destination. A store
/// an outside bundle root holds is denied there, and one under a bundle
/// root inside the worktree is denied at both places the box shows it.
#[test]
fn a_store_a_bundle_root_holds_is_denied_wherever_the_box_shows_it() {
    let Some(outside) = Stored::at("bundle/secrets.env", &[]) else {
        return;
    };
    let shown = Path::new("/runtime/bundle/secrets.env");
    outside.exec_denies(shown, Some(&outside.root.join("bundle")));
    let Some(inside) = Stored::at("work/bundle/secrets.env", &[]) else {
        return;
    };
    let bundle = inside.work.join("bundle");
    inside.exec_denies(shown, Some(&bundle));
    inside.exec_denies(&inside.store, Some(&bundle));
    for stored in [&outside, &inside] {
        assert_eq!(std::fs::read_to_string(&stored.store).unwrap(), FAKE);
    }
}

/// The host toolchain the box binds is accounted for too (ruling 7): a
/// store sealed absent below `/usr/local`, which no test can write, would
/// be shown there, so the box refuses as it refuses any absent store in
/// its reach.
#[test]
fn a_store_the_host_toolchain_would_show_is_accounted_for() {
    let Some(mut stored) = Stored::sealed("outside/secrets.env", &[]) else {
        return;
    };
    let absent = Path::new("/usr/local/brokkr-u8h-absent/secrets.env");
    stored.reseal(absent.to_path_buf());
    let local = std::fs::canonicalize("/usr/local").unwrap();
    let place = local.join("brokkr-u8h-absent/secrets.env");
    assert_eq!(stored.built(), Err(Unhidden::Absent(place)));
}

/// A store sealed in reach but absent as the box is built refuses, and
/// the box creates nothing on the host; one absent outside every mount
/// mounts nothing and the box runs.
#[test]
fn a_store_absent_in_reach_refuses_and_the_box_makes_none() {
    let Some(stored) = Stored::sealed("work/.forge/secrets.env", &[]) else {
        return;
    };
    stored.refused(&Unhidden::Absent(stored.store.clone()));
    assert!(!stored.store.exists(), "the box made no store");
    let Some(stored) = Stored::sealed("work/deep/er/secrets.env", &[]) else {
        return;
    };
    stored.refused(&Unhidden::Absent(stored.store.clone()));
    let Some(outside) = Stored::sealed("outside/secrets.env", &[]) else {
        return;
    };
    assert!(!outside.built().unwrap().join(" ").contains("/dev/null"));
    assert_eq!(outside.call("echo ran").unwrap().stdout, "ran\n");
    assert!(!outside.store.exists());
}

/// A store with a second link, wherever that link lies, and a store that
/// is no regular file each refuse with the one cause before the command
/// runs: a link in the worktree would read what the hide covers. A store
/// that cannot be observed refuses with its error kind, and one sealed as
/// spelled because no walk resolves it, through a file or a loop of links,
/// refuses as no longer resolving to itself.
#[test]
fn a_store_of_two_links_or_no_regular_file_refuses_before_the_command_runs() {
    let Some(stored) = Stored::at("outside/secrets.env", &[]) else {
        return;
    };
    std::fs::hard_link(&stored.store, stored.root.join("outside/again")).unwrap();
    stored.refused(&Unhidden::Linked(stored.store.clone()));
    let text = format!(
        "secrets store cannot be hidden from workspace hands: '{}' is not one regular file of \
         one link its user owns",
        stored.store.display()
    );
    assert_eq!(Unhidden::Linked(stored.store.clone()).to_string(), text);
    let Some(stored) = Stored::sealed("work/.forge/secrets.env", &[]) else {
        return;
    };
    std::fs::create_dir_all(&stored.store).unwrap();
    stored.refused(&Unhidden::Linked(stored.store.clone()));
    let Some(mut blocked) = Stored::sealed("work/file/secrets.env", &[]) else {
        return;
    };
    std::fs::write(blocked.work.join("file"), "").unwrap();
    let kind = std::io::ErrorKind::NotADirectory;
    blocked.refused(&Unhidden::Unobserved(blocked.store.clone(), kind));
    let unplain = blocked.work.join("file/x/../secrets.env");
    blocked.reseal(unplain.clone());
    blocked.refused(&Unhidden::Unresolved(unplain.clone()));
    std::fs::remove_file(blocked.work.join("file")).unwrap();
    std::fs::create_dir_all(blocked.work.join("file/x")).unwrap();
    planted(&blocked.work.join("file/secrets.env"), FAKE);
    blocked.refused(&Unhidden::Unresolved(unplain.clone()));
    let text = format!(
        "secrets store cannot be hidden from workspace hands: the sealed path '{}' is not \
         absolute and plain, so it no longer resolves to itself",
        unplain.display()
    );
    assert_eq!(Unhidden::Unresolved(unplain).to_string(), text);
    let Some(mut looped) = Stored::sealed("outside/secrets.env", &[]) else {
        return;
    };
    std::fs::create_dir(looped.root.join("outside")).unwrap();
    std::os::unix::fs::symlink("b", looped.root.join("outside/a")).unwrap();
    std::os::unix::fs::symlink("a", looped.root.join("outside/b")).unwrap();
    looped.reseal(looped.root.join("outside/a/secrets.env"));
    let link = looped.root.join("outside/a");
    looped.refused(&Unhidden::Routed(looped.store.clone(), link));
}

/// Operator ruling 8 of 2026-10-24: a store whose mode grants its group or
/// others any permission refuses with the one cause before the command
/// runs, and is never changed; at 0600 it is hidden.
#[test]
fn a_store_open_to_its_group_or_others_refuses_and_is_never_changed() {
    let Some(stored) = Stored::at("work/.forge/secrets.env", &[]) else {
        return;
    };
    for mode in [0o640, 0o644, 0o666] {
        let open = std::fs::Permissions::from_mode(mode);
        std::fs::set_permissions(&stored.store, open).unwrap();
        stored.refused(&Unhidden::Shared(stored.store.clone(), mode));
        let kept = std::fs::metadata(&stored.store)
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(kept & 0o7777, mode, "the store was never changed");
    }
    let text = format!(
        "secrets store cannot be hidden from workspace hands: '{}' grants its group or others \
         access (mode 0640)",
        stored.store.display()
    );
    assert_eq!(
        Unhidden::Shared(stored.store.clone(), 0o640).to_string(),
        text
    );
    planted(&stored.store, FAKE);
    stored.denies(&stored.store);
}

/// The mount table as this process reads it, with one record more: a bind
/// of the directory holding `store` at `point`, as a privileged
/// `mount --bind` makes one.
fn aliased(store: &Path, point: &Path) -> Vec<u8> {
    let (table, own) = own_record(store);
    let dir = store.parent().unwrap();
    let root = own.root.join(dir.strip_prefix(&own.point).unwrap());
    let (major, minor) = own.dev;
    let line = format!(
        "4000000000 {} {major}:{minor} {} {} rw - tmpfs alias rw\n",
        own.id,
        root.display(),
        point.display()
    );
    [table, line.into_bytes()].concat()
}

/// The mount table as this process reads it, and the record of the mount
/// `store` lies on.
fn own_record(store: &Path) -> (Vec<u8>, super::super::namespace::sources::host::Record) {
    use super::super::namespace::sources::host::records;
    use rustix::fs::{statx, AtFlags, StatxFlags, CWD};
    let table = std::fs::read("/proc/self/mountinfo").unwrap();
    let held = statx(CWD, store, AtFlags::empty(), StatxFlags::MNT_ID).unwrap();
    let all = records(&table, usize::MAX).unwrap();
    let own = all.into_iter().find(|record| record.id == held.stx_mnt_id);
    (table, own.unwrap())
}

/// A store another mount shows inside the worktree, or under a bind,
/// refuses with the one cause before the command runs, though its own
/// place lies outside every mount; one shown only outside the box's reach
/// is hidden as before, and a table that names no mount for it refuses.
#[test]
fn a_store_a_mount_shows_in_reach_refuses_before_the_command_runs() {
    let Some(stored) = Stored::at("outside/secrets.env", &[("tools", "ro")]) else {
        return;
    };
    for point in ["work/alias", "tools/alias"] {
        let point = stored.root.join(point);
        TABLE.set(Some(aliased(&stored.store, &point)));
        let shown = Unhidden::Shown(stored.store.clone(), point.join("secrets.env"));
        stored.refused(&shown);
        TABLE.set(None);
    }
    let text = format!(
        "secrets store cannot be hidden from workspace hands: a mount shows '{}' at '{}', in \
         the box's reach",
        stored.store.display(),
        stored.root.join("work/alias/secrets.env").display()
    );
    let alias = stored.root.join("work/alias/secrets.env");
    assert_eq!(
        Unhidden::Shown(stored.store.clone(), alias).to_string(),
        text
    );
    TABLE.set(Some(aliased(&stored.store, &stored.root.join("away"))));
    assert_eq!(stored.call("echo ran").unwrap().stdout, "ran\n");
    TABLE.set(Some(b"1 0 0:1 / / rw - tmpfs none rw\n".to_vec()));
    stored.refused(&Unhidden::Unmapped(stored.store.clone()));
    TABLE.set(None);
}

/// A directory mounted back onto its own place shows the store at the
/// place it already has, which is no second place: a store the worktree
/// holds is hidden as before (review M1).
#[test]
fn a_directory_mounted_back_onto_itself_shows_the_store_at_no_second_place() {
    let Some(stored) = Stored::at("work/.forge/secrets.env", &[]) else {
        return;
    };
    let folder = stored.store.parent().unwrap();
    TABLE.set(Some(aliased(&stored.store, folder)));
    stored.denies(&stored.store);
    TABLE.set(None);
}

/// The table with the record of the mount `store` lies on rewritten to the
/// device `dev` and the filesystem `kind`, alone.
fn retyped(store: &Path, dev: &str, kind: &str) -> Vec<u8> {
    let (_, own) = own_record(store);
    let (root, point) = (own.root.display(), own.point.display());
    let line = format!("{} 1 {dev} {root} {point} rw - {kind} none rw\n", own.id);
    line.into_bytes()
}

/// The store's mount is the one the table records under the store's own
/// mount id, matched by that record's own device, which a btrfs subvolume
/// reports apart from the store's `stat`, and it must be a local
/// filesystem's, as MB4's `within` proves a store's (review L1): a device
/// the store's `stat` does not report still hides it, and a FUSE mount
/// refuses.
#[test]
fn the_store_s_mount_is_the_table_s_own_on_a_local_filesystem() {
    let Some(stored) = Stored::at("work/.forge/secrets.env", &[]) else {
        return;
    };
    TABLE.set(Some(retyped(&stored.store, "4095:4095", "tmpfs")));
    stored.denies(&stored.store);
    TABLE.set(Some(retyped(&stored.store, "4095:4095", "fuse")));
    stored.refused(&Unhidden::Unmapped(stored.store.clone()));
    TABLE.set(None);
}

/// A store an overlay bind holds refuses with the one cause before the
/// command runs: a pin of its folder would bind the host's own over the
/// overlay, so its folders cannot be pinned.
#[test]
fn a_store_an_overlay_shows_refuses_before_the_command_runs() {
    let Some(stored) = Stored::at("cache/secrets.env", &[("cache", "overlay")]) else {
        return;
    };
    let overlaid = Unhidden::Overlaid(stored.store.clone(), stored.store.clone());
    stored.refused(&overlaid);
    let text = format!(
        "secrets store cannot be hidden from workspace hands: an overlay shows '{s}' at '{s}', \
         where no bind can pin its folders",
        s = stored.store.display()
    );
    assert_eq!(overlaid.to_string(), text);
}

/// A store in the Git directory, whether the worktree holds it or it lies
/// outside as a linked worktree's does, or one a folder of which holds it,
/// would have a folder pinned over the hooks and config the box hid there
/// before: it refuses with the one cause before the command runs.
#[test]
fn a_store_whose_folder_pins_would_uncover_the_git_directory_refuses() {
    let Some(mut stored) = Stored::sealed("outside/secrets.env", &[]) else {
        return;
    };
    git(&stored.work, &["init", "-q"]);
    let store = stored.work.join(".git/secrets.env");
    planted(&store, FAKE);
    stored.reseal(store);
    stored.refused(&Unhidden::Gitted(stored.store.clone()));
    let Some(mut nested) = Stored::sealed("outside/secrets.env", &[]) else {
        return;
    };
    let sub = nested.work.join("sub");
    git(&nested.work, &["init", "-q", "sub"]);
    let gitdir = format!("gitdir: {}\n", sub.join(".git").display());
    std::fs::write(nested.work.join(".git"), gitdir).unwrap();
    planted(&sub.join("secrets.env"), FAKE);
    nested.reseal(sub.join("secrets.env"));
    nested.refused(&Unhidden::Gitted(nested.store.clone()));
    let Some(mut linked) = Stored::sealed("outside/secrets.env", &[]) else {
        return;
    };
    std::fs::remove_dir(&linked.work).unwrap();
    let common = linked.root.join("repo/.git");
    linked_worktree(&linked.root.join("repo"), &linked.work);
    planted(&common.join("hooks/secrets.env"), FAKE);
    linked.reseal(common.join("hooks/secrets.env"));
    linked.refused(&Unhidden::Gitted(linked.store.clone()));
    let text = format!(
        "secrets store cannot be hidden from workspace hands: pinning the folders of '{}' would \
         uncover the hooks and config hidden in the Git directory",
        nested.store.display()
    );
    assert_eq!(Unhidden::Gitted(nested.store.clone()).to_string(), text);
}

/// Operator ruling 9 of 2026-10-24: a store the artifact root holds inside
/// the worktree refuses before the command runs with its exact cause, which
/// names the root: its read-only mount, made after the hide, would show
/// the store again.
#[test]
fn a_store_the_artifact_root_holds_refuses_before_the_command_runs() {
    let Some(mut stored) = Stored::at("work/.forge/artifacts/secrets.env", &[]) else {
        return;
    };
    stored.artifacts = artifact_root(&stored.work.join(".forge"));
    let rooted = Unhidden::Rooted(stored.store.clone(), stored.artifacts.clone());
    stored.refused(&rooted);
    assert_eq!(std::fs::read_to_string(&stored.store).unwrap(), FAKE);
    let text = format!(
        "secrets store cannot be hidden from workspace hands: the artifact root '{}' holds '{}', \
         and its read-only mount would show the store again",
        stored.artifacts.display(),
        stored.store.display()
    );
    assert_eq!(rooted.to_string(), text);
}

/// The names in `dir`, sorted: the host's entries a box must not change.
fn entries(dir: &Path) -> Vec<std::ffi::OsString> {
    let mut names: Vec<_> = (std::fs::read_dir(dir).unwrap())
        .map(|entry| entry.unwrap().file_name())
        .collect();
    names.sort();
    names
}

/// Operator rulings 12 and 15 of 2026-10-24: a store whose sealed path is
/// not valid UTF-8 refuses before the command runs whatever the box mounts,
/// outside every mount too, and so does one whose place in the box would
/// be spelled through a workdir that is not; no mount is spelled at a lossy
/// name, so nothing is made beside it on the host.
#[test]
fn a_store_whose_real_place_is_not_utf8_refuses_and_the_host_is_unchanged() {
    use std::os::unix::ffi::OsStrExt;
    let lossy = std::ffi::OsStr::from_bytes(b"secret-\xFF.env");
    let Some(mut stored) = Stored::sealed("outside/store.env", &[]) else {
        return;
    };
    let forge = stored.work.join(".forge");
    let place = forge.join(lossy);
    std::fs::create_dir_all(stored.root.join("outside")).unwrap();
    std::fs::create_dir(&forge).unwrap();
    planted(&place, FAKE);
    std::os::unix::fs::symlink(&place, &stored.store).unwrap();
    stored.reseal(stored.store.clone());
    let before = entries(&forge);
    let unspelled = Unhidden::Unspelled(place.clone());
    stored.refused(&unspelled);
    assert_eq!(entries(&forge), before, "the box made nothing on the host");
    let outside = stored.root.join("outside").join(lossy);
    planted(&outside, FAKE);
    stored.reseal(outside.clone());
    stored.refused(&Unhidden::Unspelled(outside));
    for (store, first) in [
        ("work/.forge/secrets.env", ".forge"),
        ("work/secrets.env", "secrets.env"),
    ] {
        let Some(mut spelled) = Stored::at(store, &[]) else {
            return;
        };
        let workdir = spelled.root.join(std::ffi::OsStr::from_bytes(b"wt-\xFF"));
        std::os::unix::fs::symlink(&spelled.work, &workdir).unwrap();
        spelled.work = workdir.clone();
        spelled.reseal(spelled.store.clone());
        let unspelled = Unhidden::Unspelled(workdir.join(first));
        assert_eq!(spelled.built(), Err(unspelled), "{store}");
    }
    let text = format!(
        "secrets store cannot be hidden from workspace hands: '{}' is not valid UTF-8, so no \
         mount can be spelled at it",
        place.display()
    );
    assert_eq!(unspelled.to_string(), text);
}

/// Operator ruling 13 of 2026-10-24: a hide lands where the last mount
/// over its place puts it. A read-write bind declared at `/runtime/bundle`
/// is mounted after the bundle root there, so a hide planned for a store
/// the bundle root holds would land in the bind's source instead: the box
/// refuses before the command runs, and the bundle root is unchanged. No
/// test can make `/runtime/bundle` on the host, so the bind's source is
/// absent, and the box is judged on the mounts it declares.
#[test]
fn a_mount_that_would_shadow_a_hide_refuses_and_the_host_is_unchanged() {
    let Some(mut stored) = Stored::at("bundle/secrets.env", &[]) else {
        return;
    };
    let bound = json!({"kind": "workspace", "binds": [{"path": SANDBOX_BUNDLE, "mode": "rw"}]});
    stored.spec = spec_of(bound);
    stored.reseal(stored.store.clone());
    let bundle = stored.root.join("bundle");
    let before = entries(&bundle);
    let shown = Path::new(SANDBOX_BUNDLE).join("secrets.env");
    let shadowed = Unhidden::Shadowed(stored.store.clone(), shown.clone());
    let text = format!("hands namespace: {shadowed}");
    assert_eq!(stored.exec_in("touch ran", Some(&bundle)), Err(text));
    assert!(!stored.work.join("ran").exists(), "the command never ran");
    assert_eq!(entries(&bundle), before);
    let text = format!(
        "secrets store cannot be hidden from workspace hands: a later mount shadows '{}', where \
         the box would hide '{}'",
        shown.display(),
        stored.store.display()
    );
    assert_eq!(shadowed.to_string(), text);
}

/// A store replaced at its name once the box hid it, before its argv is
/// final, refuses with the one cause before the command runs: the second
/// observation is not the first.
#[test]
fn a_store_replaced_between_the_box_observations_refuses() {
    let Some(stored) = Stored::at("work/.forge/secrets.env", &[]) else {
        return;
    };
    let (store, new) = (stored.store.clone(), stored.root.join("new.env"));
    CONFIRMING.set(Some(Box::new(move || {
        planted(&new, FAKE);
        std::fs::rename(&new, &store).unwrap();
    })));
    let changed = Unhidden::Changed(stored.store.clone());
    let text = format!("hands namespace: {changed}");
    assert_eq!(stored.call("touch ran").map(|ran| ran.exit_code), Err(text));
    assert!(!stored.work.join("ran").exists(), "the command never ran");
    let text = format!(
        "secrets store cannot be hidden from workspace hands: '{}' changed between the box's \
         two observations",
        stored.store.display()
    );
    assert_eq!(changed.to_string(), text);
    stored.denies(&stored.store);
}

/// Operator ruling 3 of 2026-10-23, as Linux does it: a store renamed over
/// on the host while one workspace call runs detaches that box's hide, so
/// that one call reads the new store; the next call hides it again.
#[test]
fn a_store_renamed_over_during_one_box_shows_that_box_the_new_store() {
    let Some(stored) = Stored::at("work/.forge/secrets.env", &[]) else {
        return;
    };
    let path = stored.store.display();
    let command = format!(
        "cat {path} 2>/dev/null; echo first; touch ready; \
         i=0; while [ ! -e go ] && [ $i -lt 1500 ]; do sleep 0.02; i=$((i+1)); done; \
         cat {path} 2>/dev/null; echo second"
    );
    let (work, store, root) = (
        stored.work.clone(),
        stored.store.clone(),
        stored.root.clone(),
    );
    let renaming = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(30);
        while !work.join("ready").exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        let new = root.join("new.env");
        planted(&new, "DOCS_TOKEN=renamed\n");
        std::fs::rename(&new, &store).unwrap();
        std::fs::write(work.join("go"), "").unwrap();
    });
    let ran = stored.call(&command).unwrap();
    renaming.join().unwrap();
    assert_eq!(ran.stdout, "first\nDOCS_TOKEN=renamed\nsecond\n");
    let ran = stored.call(&format!("cat {path}")).unwrap();
    assert_eq!(ran.exit_code, 1);
    denied(&stored.store, &ran.stderr);
}
