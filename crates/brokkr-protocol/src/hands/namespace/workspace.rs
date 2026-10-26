//! The workspace box's own mounts and environment, in order (decision 0043;
//! U8a, U8a2c, U8h, U8k): the namespace, its toolchain and its identity are
//! [`Namespace`]'s; the private home and tmp, the worktree, the bundle, the
//! Git directories, the declared binds and the artifact root are ordered
//! here. Each sealed source is mounted from the handle opened at the box's
//! last observation, and the box carries those handles, and no other, into
//! bubblewrap ([`Boxed::launch`]).

use std::os::fd::{AsRawFd, OwnedFd, RawFd};
#[cfg(target_os = "linux")]
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::super::environment::box_environment;
use super::super::{evidence, sealed, session, HandsSpec, OverlayWrites, Sealed, SANDBOX_BUNDLE};
use super::{namespace_join, namespace_path, Mount, Namespace, Profile, SANDBOX_HOME};

/// What one workspace box is built from. `artifacts` is the engine's
/// run-local artifact root, which the box never writes (U8a). `scratch`
/// holds this call's generated identity files and private home and tmp;
/// `writes` says where the upper layers of overlay binds live: a session's
/// are the seat's to remove when it ends. `sealed` is the reach the box is
/// built from (U8a2c), its `~/` the sealed HOME; `identity` who the seat
/// commits as; `bundle_root` the bundle an exec box binds read-only.
pub(in crate::hands) struct Workspace<'a> {
    pub(in crate::hands) spec: &'a HandsSpec,
    pub(in crate::hands) workdir: &'a Path,
    pub(in crate::hands) artifacts: &'a Path,
    pub(in crate::hands) sealed: &'a Sealed,
    pub(in crate::hands) scratch: &'a Path,
    pub(in crate::hands) writes: OverlayWrites<'a>,
    pub(in crate::hands) identity: &'a [(String, String)],
    pub(in crate::hands) bundle_root: Option<&'a Path>,
}

/// A workspace box ready to enter: its argv, and the handle of each source
/// the argv mounts by descriptor, by its host path, held until bubblewrap
/// takes them (U8k).
#[derive(Debug)]
pub(in crate::hands) struct Boxed {
    argv: Vec<String>,
    held: Vec<(PathBuf, OwnedFd)>,
}

impl Workspace<'_> {
    /// The box that runs `command`: the namespace, the binds, the
    /// environment, then `--` and the command.
    pub(in crate::hands) fn boxed(&self, command: &[String]) -> std::io::Result<Boxed> {
        let Workspace {
            spec,
            sealed,
            scratch,
            bundle_root,
            ..
        } = *self;
        // The workdir is bound at its own path, and that path must be
        // absolute: `--repo .` is the common spelling, and a relative
        // destination is resolved by bwrap against the box's root, so the
        // worktree lands over `/` and shadows `/runtime` before the bundle
        // mounts there — the first weekly research sweep parked on exactly
        // that, "Can't mkdir parents for /runtime/bundle". Symlinks stay as
        // the host spells them; only the relative prefix is resolved.
        let workdir = &std::path::absolute(self.workdir)?;
        let private_home = scratch.join("home");
        let private_tmp = scratch.join("tmp");
        let mut namespace = Namespace::open(Profile::Workspace, spec.network);
        namespace.identity(&scratch.join("etc"))?;
        session::private_dir(&private_home)?;
        session::private_dir(&private_tmp)?;
        // U8a: the engine's own facts of the evidence, observed before any
        // mount and never authored; no writable mount may alias it.
        let mut protected = evidence::Protected::observe(self.artifacts, workdir)?;
        // U8a2c: the reach sealed for the box, observed again before any mount.
        sealed.confirm(spec, workdir)?;
        for private in [&private_home, &private_tmp] {
            protected.exclude(private)?;
        }
        // The private home and tmp go in BEFORE the worktree, so a worktree
        // that itself lives under /tmp is mounted on top of the private /tmp
        // rather than hidden beneath it.
        namespace.mount(Mount::Bind, &private_home, SANDBOX_HOME);
        namespace.mount(Mount::Bind, &private_tmp, "/tmp");
        let worktree = sealed.worktree();
        namespace.mount(Mount::Bind, worktree, &namespace_path(workdir));
        if let Some(bundle_root) = bundle_root {
            let target = namespace_path(Path::new(SANDBOX_BUNDLE));
            namespace.mount(Mount::RoBind, bundle_root, &target);
        }
        git(&mut namespace, &mut protected, sealed, workdir)?;
        // U8a, U8h: each directory between the worktree and an artifact root
        // or a secrets store it holds is a mount of its own, so no write in the
        // box renames it, both chains pinned together, outermost first; the
        // binds follow, and the store is hidden after every writable mount.
        let seen = (workdir.as_path(), bundle_root);
        let writes = self.writes;
        let store =
            sealed::mount_binds(&mut namespace, &mut protected, spec, sealed, seen, writes)?;
        // U8a: a root the worktree holds read-only, after every writable
        // mount; one outside every writable mount is never mounted at all.
        if let Some(root) = protected.held() {
            namespace.mount_in_place(Mount::RoBind, root);
        }
        for (key, value) in box_environment(spec, sealed.home()) {
            namespace.setenv(key, &value);
        }
        for (key, value) in self.identity {
            namespace.setenv(key, value);
        }
        // U8a (L1), U8a2c: what was observed is observed again as the argv is
        // final, the evidence, the sealed reach and its hidden store alike (U8h).
        protected.confirm()?;
        store.confirm(sealed, spec, workdir)?;
        // U8k: each sealed source opened once, at that last observation, and
        // mounted from its handle, so bubblewrap never names it again; and
        // what the box mounts from inside one, Git's config, the pins and
        // the artifact root, opened beneath that handle as it was observed.
        let mut opened = sealed.opened()?;
        let inner = sealed.retained().into_iter();
        let inner = inner.chain(store.beneath(sealed, workdir)).collect();
        opened.extend(sealed::beneath(&opened, inner)?);
        let held = namespace.held(opened)?;
        let argv = namespace.enter(workdir, command);
        Ok(Boxed { argv, held })
    }
}

/// Ruling 6: the git directory. A `git worktree`'s lives outside the
/// worktree and is bound so git works at all; either way its `hooks` are
/// hidden behind an empty tmpfs and its `config` is read-only, so nothing a
/// boxed command writes can become a program the host runs on its next git
/// invocation. The per-worktree directory lies under the shared one for a
/// linked worktree, so binding the common directory covers both.
///
/// KNOWN OPEN, and much wider than the harness runner beside it. Binding
/// the whole common directory read-write leaves four things open that the
/// dsh runner closes:
///
///   * every worktree's `config.worktree`, `commondir` and `gitdir`, so in
///     a repository carrying `extensions.worktreeConfig` a boxed command
///     can write `<common>/worktrees/<name>/config.worktree` and the host's
///     next `git` there honours a `core.hooksPath` the box chose;
///   * the shared REF store — `refs`, `packed-refs`, `logs` — so a boxed
///     command can move a branch a sibling worktree has checked out, at
///     every spelling git uses;
///   * the shared OBJECT store, which a boxed command can destroy or
///     corrupt, and `objects/info/alternates`, which it can repoint;
///   * every sibling worktree's administrative directory.
///
/// The dsh runner answers all four by giving the seat a PRIVATE common
/// directory and promoting one ref afterwards (`dsh_sandbox`, decision
/// 0054). Narrowing THIS write set the same way is a change under decision
/// 0043 and takes its own number — see 0054's consequences, which record it
/// rather than fixing it silently.
///
/// U8a2c: both directories are the sealed reach's, never Git's answer now;
/// one inside the worktree is where the box sees the worktree. U8k: the
/// `config` is mounted from a handle opened beneath the common directory's
/// own, and one inside the worktree is held to the very directory sealed
/// ([`Sealed::retained`]), so the protection lands on the directory the
/// box retains, or the box refuses; the `hooks` tmpfs is made inside the
/// box, on that directory as the box mounts it.
fn git(
    namespace: &mut Namespace,
    protected: &mut evidence::Protected,
    sealed: &Sealed,
    workdir: &Path,
) -> std::io::Result<()> {
    let Some(common) = sealed.common() else {
        return Ok(());
    };
    let worktree = sealed.worktree();
    let inside = common
        .strip_prefix(worktree)
        .ok()
        .map(|rest| workdir.join(rest));
    if inside.is_none() {
        protected.exclude(common)?;
        namespace.mount_in_place(Mount::Bind, common);
    }
    let seen = inside.unwrap_or_else(|| common.to_path_buf());
    namespace.tmpfs(&namespace_join(&namespace_path(&seen), Path::new("hooks")));
    let config = namespace_path(&seen.join("config"));
    namespace.mount(Mount::RoBind, &common.join("config"), &config);
    Ok(())
}

impl Boxed {
    /// The argv alone. Its descriptors close with the box's handles, so it
    /// names every mount the box makes and can enter none of them.
    #[cfg(test)]
    pub(in crate::hands) fn into_argv(self) -> Vec<String> {
        self.argv
    }

    /// What `start` makes of `bwrap` set to enter the box, its arguments
    /// the argv's after its name: on Linux each handle the argv mounts is
    /// left open across the exec in the started process alone, every other
    /// descriptor closed by it (`entry::inheritor`), and the handles stay
    /// open until `start` returns, so no number the argv names is another
    /// file's by then. Off Linux no bubblewrap runs, and none is carried.
    pub(in crate::hands) fn launch<T>(
        &self,
        bwrap: &Path,
        start: impl FnOnce(&mut Command) -> T,
    ) -> T {
        let mut command = Command::new(bwrap);
        command.args(&self.argv[1..]);
        let fds: Vec<RawFd> = self.held.iter().map(|(_, fd)| fd.as_raw_fd()).collect();
        // SAFETY: the hook runs between fork and exec and makes system calls
        // on descriptors alone, which `self` holds open meanwhile.
        #[cfg(target_os = "linux")]
        unsafe {
            command.pre_exec(super::entry::inheritor(fds))
        };
        #[cfg(not(target_os = "linux"))]
        drop(fds);
        start(&mut command)
    }

    /// Each handle the argv mounts, by the host path it was opened at.
    #[cfg(test)]
    pub(in crate::hands) fn held(&self) -> &[(PathBuf, OwnedFd)] {
        &self.held
    }

    /// The argv, its descriptors numbered as the box holds them.
    #[cfg(test)]
    pub(in crate::hands) fn argv(&self) -> &[String] {
        &self.argv
    }
}
