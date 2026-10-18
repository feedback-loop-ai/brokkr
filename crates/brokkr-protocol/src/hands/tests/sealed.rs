//! Every workspace box built from the reach sealed at admission (decision
//! 0065 slice two, U8a2c; CR2, MB3): each bind from its sealed canonical
//! source at its authored spelling, the Git directories from the sealed
//! reach, and, before each box starts and again once its argv is final,
//! the reach observed again, a source, an ancestor of one or the
//! worktree's Git pointer that no longer resolves to its sealed identity
//! refusing with MB3's identity cause before anything spawns. In real
//! namespaces, a writer's own box redirecting a later box of its own is
//! refused at the next workspace call and the following exec alike, while
//! an unchanged writer's calls, worktree writes and commits run.

use super::super::evidence::CONFIRMING;
use super::super::sealed::Drifted;
use super::*;

/// MB3's refusal of a box built from `path`, which no longer resolves as
/// it was sealed, in the words a workspace call and an exec return it.
fn moved(path: &Path) -> String {
    format!(
        "hands namespace: MCP server box filesystem identity is not protected: '{}', which the \
         box is built from, no longer resolves as it was sealed",
        path.display()
    )
}

/// The refusal a box build returned, as the drift it is.
fn drifted_of(built: std::io::Result<Vec<String>>) -> Drifted {
    let error = built.expect_err("the box was built");
    *error.into_inner().unwrap().downcast::<Drifted>().unwrap()
}

/// The argv of a box of `spec` in `work`, built from `reach`, its private
/// tree and session under `dir`.
fn built(
    spec: &HandsSpec,
    work: &Path,
    reach: &Sealed,
    dir: &Path,
) -> std::io::Result<Vec<String>> {
    boxed_from(spec, work, &artifact_root(dir), reach, dir)
}

/// Under `root`, a repository `main` with a linked worktree of it,
/// `linked`, beside another repository, `other`: `main`, `other` and
/// `linked`.
fn beside_another(root: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let (main, other, linked) = (root.join("main"), root.join("other"), root.join("linked"));
    linked_worktree(&main, &linked);
    committed(&other);
    (main, other, linked)
}

/// Under `root`, a directory `tools` and hands binding it read-only.
fn tools_bound(root: &Path) -> (PathBuf, HandsSpec) {
    let tools = root.join("tools");
    std::fs::create_dir(&tools).unwrap();
    let spec = binding(&[(&tools, "ro")]);
    (tools, spec)
}

/// Whether the box argv `text` binds the Git common directory `common`
/// read-write at its own path.
fn binds_in_place(text: &str, common: &Path) -> bool {
    text.contains(&format!("--bind {c} {c}", c = common.display()))
}

/// A spec binding each `(path, mode)` as authored.
fn binding(binds: &[(&Path, &str)]) -> HandsSpec {
    let binds: Vec<Value> = (binds.iter())
        .map(|(path, mode)| json!({"path": path, "mode": mode}))
        .collect();
    spec_of(json!({"kind": "workspace", "binds": binds}))
}

/// Each bind is mounted from the source sealed for it, its canonical path,
/// at the spelling its authored path names, in every mode: a source
/// authored through a link is mounted from what the link named when it was
/// sealed, never resolved again from the spelling.
#[test]
fn every_bind_is_mounted_from_its_sealed_source_at_its_authored_spelling() {
    let (dir, root, work) = fixture();
    let (real, link) = (root.join("real"), root.join("link"));
    for mode in ["ro", "rw", "overlay"] {
        std::fs::create_dir_all(real.join(mode)).unwrap();
    }
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let (ro, rw, overlay) = (link.join("ro"), link.join("rw"), link.join("overlay"));
    let spec = binding(&[(&ro, "ro"), (&rw, "rw"), (&overlay, "overlay")]);
    let reach = sealed(&spec, &work, &root.join("home"));
    let text = built(&spec, &work, &reach, dir.path()).unwrap().join(" ");
    let (r, l) = (real.display(), link.display());
    assert!(
        text.contains(&format!("--ro-bind-try {r}/ro {l}/ro")),
        "{text}"
    );
    assert!(
        text.contains(&format!("--bind-try {r}/rw {l}/rw")),
        "{text}"
    );
    let upper = dir.path().join("session/overlay/2");
    let layers = format!("--overlay {u}/upper {u}/work", u = upper.display());
    let overlaid = format!("--overlay-src {r}/overlay {layers} {l}/overlay");
    assert!(text.contains(&overlaid), "{text}");
    assert!(
        text.contains(&format!("--bind {w} {w}", w = work.display())),
        "{text}"
    );
}

/// The Git directories a box binds are the sealed reach's: a linked
/// worktree's common directory outside it is mounted from the path sealed,
/// its hooks hidden and its config read-only.
#[test]
fn the_git_directories_a_box_binds_are_those_sealed() {
    let (dir, root, _) = fixture();
    let (main, linked) = (root.join("main"), root.join("linked"));
    linked_worktree(&main, &linked);
    let spec = HandsSpec::default();
    let reach = sealed(&spec, &linked, &root.join("home"));
    let text = built(&spec, &linked, &reach, dir.path()).unwrap().join(" ");
    let common = main.join(".git");
    assert!(binds_in_place(&text, &common), "{text}");
    assert!(
        text.contains(&format!("--tmpfs {}/hooks", common.display())),
        "{text}"
    );
    let config = format!("--ro-bind-try {c}/config {c}/config", c = common.display());
    assert!(text.contains(&config), "{text}");
}

/// Git is never asked where a box's Git directories are: where the
/// worktree's `commondir` names another repository between the reach's
/// observation and the box's mounts, and its own again before the argv is
/// final, the box binds the common directory sealed, not Git's answer.
#[test]
fn a_box_binds_the_sealed_common_directory_whatever_git_answers_meanwhile() {
    let (dir, root, _) = fixture();
    let (main, other, linked) = beside_another(&root);
    let spec = HandsSpec::default();
    let reach = sealed(&spec, &linked, &root.join("home"));
    let commondir = main.join(".git/worktrees/linked/commondir");
    let (was, elsewhere) = (std::fs::read(&commondir).unwrap(), other.join(".git"));
    let redirected = (commondir.clone(), format!("{}\n", elsewhere.display()));
    super::super::sealed::CONFIRMED.set(Some(Box::new(move || {
        std::fs::write(redirected.0, redirected.1).unwrap()
    })));
    CONFIRMING.set(Some(Box::new(move || {
        std::fs::write(commondir, was).unwrap()
    })));
    let text = built(&spec, &linked, &reach, dir.path()).unwrap().join(" ");
    assert!(binds_in_place(&text, &main.join(".git")), "{text}");
    assert!(!text.contains(&elsewhere.display().to_string()), "{text}");
}

/// A reach that does not seal each bind the box declares, or that seals
/// no worktree, builds no box: MB3's identity cause, nothing derived again.
#[test]
fn a_reach_that_does_not_seal_the_box_refuses_it() {
    let (dir, root, work) = fixture();
    let (_, spec) = tools_bound(&root);
    let bare = sealed(&HandsSpec::default(), &work, &root.join("home"));
    assert_eq!(
        drifted_of(built(&spec, &work, &bare, dir.path())),
        Drifted::Unsealed
    );
    let nothing = Sealed::default();
    let none = HandsSpec::default();
    assert_eq!(
        drifted_of(built(&none, &work, &nothing, dir.path())),
        Drifted::Unsealed
    );
    let text = "MCP server box filesystem identity is not protected: the box's reach was never \
                sealed for the hands it declares";
    assert_eq!(Drifted::Unsealed.to_string(), text);
}

/// Observed again before the box starts, a sealed source that is replaced,
/// an ancestor of one that is, one absent then that stands now, and the
/// worktree's Git pointer or its `commondir` naming another repository
/// each refuse by the path sealed in its place; the reach as sealed builds.
#[test]
fn a_reach_that_no_longer_resolves_as_sealed_refuses_before_the_box_starts() {
    let (dir, root, _) = fixture();
    let (main, other, linked) = beside_another(&root);
    let (deep, absent) = (linked.join("vendor/tools"), linked.join("cache"));
    std::fs::create_dir_all(&deep).unwrap();
    let spec = binding(&[(&deep, "ro"), (&absent, "rw")]);
    let reach = sealed(&spec, &linked, &root.join("home"));
    let build = || built(&spec, &linked, &reach, dir.path());
    assert!(build().is_ok(), "the reach as sealed builds");
    // An ancestor of a source replaced by a link toward elsewhere.
    let vendor = linked.join("vendor");
    std::fs::rename(&vendor, root.join("vendor.old")).unwrap();
    std::fs::create_dir_all(root.join("elsewhere/tools")).unwrap();
    std::os::unix::fs::symlink(root.join("elsewhere"), &vendor).unwrap();
    assert_eq!(drifted_of(build()), Drifted::Moved(deep.clone()));
    // Renamed back to the very directory it was, the reach builds again.
    std::fs::remove_file(&vendor).unwrap();
    std::fs::rename(root.join("vendor.old"), &vendor).unwrap();
    assert!(build().is_ok(), "the very directory again");
    // Another directory put at its path: the same spelling, another inode.
    std::fs::rename(&deep, root.join("tools.old")).unwrap();
    std::fs::create_dir(&deep).unwrap();
    assert_eq!(drifted_of(build()), Drifted::Moved(deep.clone()));
    std::fs::remove_dir(&deep).unwrap();
    std::fs::rename(root.join("tools.old"), &deep).unwrap();
    // A source absent when sealed that stands now.
    std::fs::create_dir(&absent).unwrap();
    assert_eq!(drifted_of(build()), Drifted::Moved(absent.clone()));
    std::fs::remove_dir(&absent).unwrap();
    assert!(build().is_ok(), "absent again");
    // The worktree's Git pointer, and its per-worktree `commondir`.
    let pointer = linked.join(".git");
    let was = std::fs::read(&pointer).unwrap();
    let elsewhere = format!("gitdir: {}\n", other.join(".git").display());
    std::fs::write(&pointer, elsewhere).unwrap();
    let git_dir = main.join(".git/worktrees/linked");
    assert_eq!(drifted_of(build()), Drifted::Moved(git_dir.clone()));
    std::fs::write(&pointer, was).unwrap();
    std::fs::write(
        git_dir.join("commondir"),
        format!("{}\n", other.join(".git").display()),
    )
    .unwrap();
    assert_eq!(drifted_of(build()), Drifted::Moved(main.join(".git")));
}

/// The reach is observed again once the box's argv is final, as U8a's
/// confirm observes the evidence: a source replaced after the box started
/// building and before its argv is final refuses it (L1).
#[test]
fn a_source_replaced_as_the_argv_is_made_final_refuses_the_box() {
    let (dir, root, work) = fixture();
    let (tools, spec) = tools_bound(&root);
    let reach = sealed(&spec, &work, &root.join("home"));
    let (moved_to, swapped) = (root.join("tools.old"), tools.clone());
    CONFIRMING.set(Some(Box::new(move || {
        std::fs::rename(&swapped, moved_to).unwrap();
        std::fs::create_dir(&swapped).unwrap();
    })));
    assert_eq!(
        drifted_of(built(&spec, &work, &reach, dir.path())),
        Drifted::Moved(tools)
    );
}

/// A writer admitted in real namespaces (MB3's proof): a linked worktree
/// of `main` beside another repository, its hands binding `vendor/tools`
/// read-only and `cache`, absent, read-write beneath it, its reach sealed,
/// and protected evidence and a coordination root outside the worktree.
#[cfg(target_os = "linux")]
struct Admitted {
    dir: tempfile::TempDir,
    root: PathBuf,
    linked: PathBuf,
    spec: HandsSpec,
    reach: Sealed,
    session: Session,
}

#[cfg(target_os = "linux")]
impl Admitted {
    /// The writer, or none where this host cannot make a namespace.
    fn new() -> Option<Admitted> {
        if !can_create_namespace() {
            let reason = "this environment cannot create a bubblewrap namespace";
            skip_boundary_proof(boundary_evidence_required(), reason);
            return None;
        }
        let (dir, root, _) = fixture();
        let (_, _, linked) = beside_another(&root);
        std::fs::create_dir_all(root.join("evidence/tools")).unwrap();
        std::fs::write(root.join("evidence/tools/artifact"), "evidence\n").unwrap();
        std::fs::create_dir(root.join("coordination")).unwrap();
        std::fs::write(root.join("coordination/reservation.json"), "{}\n").unwrap();
        std::fs::create_dir_all(linked.join("vendor/tools")).unwrap();
        let (deep, absent) = (linked.join("vendor/tools"), linked.join("cache"));
        let spec = binding(&[(&deep, "ro"), (&absent, "rw")]);
        let reach = sealed(&spec, &linked, &root.join("home"));
        let session = Session::create_under(dir.path(), "test").unwrap();
        Some(Admitted {
            dir,
            root,
            linked,
            spec,
            reach,
            session,
        })
    }

    /// The writer's next workspace call, running `command` in its box.
    fn call(&self, command: &str) -> Result<Executed, String> {
        let (artifacts, long) = (artifact_root(self.dir.path()), Duration::from_secs(30));
        let at = self.session.path();
        execute(
            &self.spec,
            &self.linked,
            &artifacts,
            &self.reach,
            at,
            command,
            long,
        )
    }

    /// The writer's next exec, boxed whole.
    fn exec(&self) -> Result<i32, String> {
        let (artifacts, bwrap) = (artifact_root(self.dir.path()), require_bwrap().unwrap());
        let (spec, work, reach) = (&self.spec, &self.linked, &self.reach);
        let session = self.dir.path().join("exec");
        run_boxed_in(
            &bwrap,
            spec,
            work,
            &artifacts,
            reach,
            &session,
            &[],
            None,
            &one("true"),
        )
    }

    /// `command`, run by the writer's own box, which must run it.
    fn attacks(&self, command: &str) {
        let ran = self.call(command).unwrap();
        assert_eq!(ran.exit_code, 0, "{command}: {}", ran.render());
    }

    /// The writer's next workspace call and the following exec, each
    /// refused before spawn by MB3's cause for `path`.
    fn refused(&self, path: &Path, how: &str) {
        let called = self.call("true").map(|ran| ran.exit_code);
        assert_eq!(called, Err(moved(path)), "{how}");
        assert_eq!(self.exec(), Err(moved(path)), "{how}");
    }

    /// What no box of the writer may change: the evidence, the
    /// coordination root's record, and the other repository's HEAD.
    fn protected(&self) -> [Vec<u8>; 3] {
        let read = |path: &str| std::fs::read(self.root.join(path)).unwrap();
        let held = ["evidence/tools/artifact", "coordination/reservation.json"];
        [read(held[0]), read(held[1]), read("other/.git/HEAD")]
    }
}

/// In real namespaces (MB3's proof): after admission sealed its reach, an
/// unchanged writer's repeated calls, worktree writes, commits and exec
/// run; then its own box replaces an ancestor of a bind source beneath its
/// worktree by a link toward protected evidence, puts a link toward the
/// coordination root where a source stood absent, renames another
/// directory into that source's place, and hard-links a file of its
/// worktree there. Each time its next workspace call
/// and the following exec refuse before spawn with MB3's exact cause, the
/// protected bytes unchanged, and once undone it runs again.
#[cfg(target_os = "linux")]
#[test]
fn a_writers_own_box_cannot_redirect_a_later_box_through_a_source() {
    let Some(writer) = Admitted::new() else {
        return;
    };
    for _ in 0..2 {
        assert_eq!(writer.call("true").unwrap().exit_code, 0);
    }
    let commit = "git -c user.name=Seat -c user.email=s@x commit -qm boxed";
    writer.attacks(&format!("echo w > made && git add made && {commit}"));
    assert_eq!(
        std::fs::read_to_string(writer.linked.join("made")).unwrap(),
        "w\n"
    );
    assert_eq!(writer.exec(), Ok(0));
    let before = writer.protected();
    let (evidence, coordination) = (
        writer.root.join("evidence"),
        writer.root.join("coordination"),
    );
    let (deep, absent) = (
        writer.linked.join("vendor/tools"),
        writer.linked.join("cache"),
    );
    let attacks = [
        (
            format!(
                "mv vendor vendor.old && ln -s {} vendor",
                evidence.display()
            ),
            &deep,
            "rm vendor && mv vendor.old vendor",
        ),
        (
            format!("ln -s {} cache", coordination.display()),
            &absent,
            "rm cache",
        ),
        (
            "mkdir decoy && mv decoy cache".to_string(),
            &absent,
            "rmdir cache",
        ),
        ("ln made cache".to_string(), &absent, "rm cache"),
    ];
    for (attack, path, undo) in attacks {
        writer.attacks(&attack);
        writer.refused(path, &attack);
        assert_eq!(writer.protected(), before, "{attack}");
        let mut undone = Command::new("sh");
        let undone = undone
            .args(["-c", undo])
            .current_dir(&writer.linked)
            .status();
        assert!(undone.unwrap().success(), "{undo}");
        assert_eq!(
            writer.call("true").unwrap().exit_code,
            0,
            "{attack}: undone"
        );
    }
}

/// In real namespaces (MB3's proof): a writer's own box rewrites the
/// worktree's Git pointer, and then its per-worktree `commondir`, to name
/// another repository's Git directory; its next workspace call and the
/// following exec each refuse before spawn with MB3's exact cause, by the
/// Git directory, then the Git common directory, sealed, the other
/// repository unchanged.
#[cfg(target_os = "linux")]
#[test]
fn a_writers_own_box_cannot_redirect_a_later_box_through_its_git_pointer() {
    let Some(writer) = Admitted::new() else {
        return;
    };
    let before = writer.protected();
    let (main, other) = (
        writer.root.join("main/.git"),
        writer.root.join("other/.git"),
    );
    let git_dir = main.join("worktrees/linked");
    writer.attacks(&format!("printf 'gitdir: {}\\n' > .git", other.display()));
    writer.refused(&git_dir, "the Git pointer");
    let pointer = format!("gitdir: {}\n", git_dir.display());
    std::fs::write(writer.linked.join(".git"), pointer).unwrap();
    let commondir = git_dir.join("commondir");
    writer.attacks(&format!(
        "printf '{}\\n' > {}",
        other.display(),
        commondir.display()
    ));
    writer.refused(&main, "its commondir");
    assert_eq!(writer.protected(), before, "the other repository unchanged");
}
