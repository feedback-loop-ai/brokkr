//! The namespace a box is built in, and the bubblewrap argv that says so
//! (decision 0065 slice two, U6c3 and U6c4): the isolation flags, the empty
//! root's skeleton, the host system set read-only, the generated identity,
//! and the environment and command that close the argv. What a box mounts
//! beyond that is its profile's: the workspace's workdir, git and declared
//! binds stay with `box_argv`; one MCP server's program tree, bootstrap and
//! private directories are [`ServerBox`]'s, every source mounted from the
//! handle its observer checked (U6c5a).

use std::ffi::OsStr;
use std::os::fd::{AsRawFd, OwnedFd};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use super::{ids, Session, HANDS_BOX_ENV, HOST_TOOLCHAIN_BINDS, SANDBOX_HOME};
use crate::broker::{Network, Reach, Refusal, Sources, Tree};

#[cfg(target_os = "linux")]
pub(super) mod sources;

/// Render a host path as a path inside the box. Paths inside the namespace
/// are POSIX paths, never host paths.
pub(super) fn namespace_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/")
}

/// Append a host-relative path below a fixed path inside the box without
/// letting the host choose the separator.
pub fn namespace_join(root: &str, relative: &Path) -> String {
    format!(
        "{}/{}",
        root.trim_end_matches('/'),
        namespace_path(relative).trim_start_matches('/')
    )
}

/// How a host path is mounted into the box. The `-try` forms skip an
/// absent source rather than fail; the others refuse to build without it.
#[derive(Debug, Clone, Copy)]
pub(super) enum Mount {
    Bind,
    BindTry,
    RoBind,
    RoBindTry,
}

impl Mount {
    fn flag(self) -> &'static str {
        match self {
            Mount::Bind => "--bind",
            Mount::BindTry => "--bind-try",
            Mount::RoBind => "--ro-bind",
            Mount::RoBindTry => "--ro-bind-try",
        }
    }
}

/// Which box the builder opens (U6c4): the workspace box the hands and an
/// exec seat run in, or the box one MCP server runs in.
#[derive(Debug, Clone, Copy)]
pub(super) enum Profile {
    Workspace,
    Server,
}

impl Profile {
    /// The read-only system set this profile binds: the shared table,
    /// where a server keeps of `/etc/ssl` only its public `certs`, so
    /// `/etc/ssl/private` and every other sibling stays outside its box
    /// (MB3). A projection of the one table, never a second list.
    fn system(self) -> impl Iterator<Item = &'static str> {
        HOST_TOOLCHAIN_BINDS
            .iter()
            .map(move |host| match (self, *host) {
                (Profile::Server, "/etc/ssl") => "/etc/ssl/certs",
                (Profile::Server | Profile::Workspace, host) => host,
            })
    }
}

/// How a box resolves host names (MB3): from its generated files alone,
/// or from them and then the host's DNS through the host's
/// `/etc/resolv.conf`, the one file of the host's `/etc` it then binds.
#[derive(Debug, Clone, Copy)]
pub(super) enum Resolver {
    Files,
    Dns,
}

/// The one host resolver file a networked server box binds.
const RESOLV_CONF: &str = "/etc/resolv.conf";

/// One box's bubblewrap argv, built in mount order: a later mount shadows
/// an earlier one, so the order the calls are made in is the boundary.
/// Every host source and box destination a mount names is kept beside it,
/// so what a profile checks is exactly what it mounts, and every path the
/// box sets up, mounted or made, so nothing later lands over one. Each
/// mount's place in the argv is kept too, so a server box can mount its
/// sources from their checked handles instead.
#[derive(Debug)]
pub(super) struct Namespace {
    argv: Vec<String>,
    paths: Vec<PathBuf>,
    targets: Vec<PathBuf>,
    binds: Vec<Bind>,
}

/// One host mount: where its flag lies in the argv, and its host source.
#[derive(Debug)]
struct Bind {
    at: usize,
    host: PathBuf,
}

impl Namespace {
    /// The isolation every box opens with, the empty root's skeleton, and
    /// the profile's system set read-only where it exists (`-try`: an
    /// absent source is skipped, never an error and never replaced by a
    /// broader one — /lib64 is a Debian fact, not a law). The network is
    /// unshared unless `network` grants it. A workspace box is a session
    /// of its own; a server box stays in the broker's, so the attempt's
    /// group cleanup reaches it (MB5).
    pub(super) fn open(profile: Profile, network: bool) -> Namespace {
        let session = match profile {
            Profile::Workspace => Some("--new-session"),
            Profile::Server => None,
        };
        let isolation = [
            "--unshare-pid",
            "--unshare-ipc",
            "--unshare-uts",
            "--unshare-cgroup-try",
            "--cap-drop",
            "ALL",
        ];
        let mut argv: Vec<String> = ["bwrap", "--die-with-parent"]
            .into_iter()
            .chain(session)
            .chain(isolation)
            .map(String::from)
            .collect();
        if !network {
            argv.push("--unshare-net".to_string());
        }
        argv.extend(["--clearenv", "--setenv", HANDS_BOX_ENV, "1"].map(String::from));
        let mut namespace = Namespace {
            argv,
            paths: Vec::new(),
            targets: Vec::new(),
            binds: Vec::new(),
        };
        namespace.made("--proc", "/proc");
        namespace.made("--dev", "/dev");
        for dir in ["/runtime", "/etc", "/home", "/root", "/run", "/usr"] {
            namespace.made("--dir", dir);
        }
        for host in profile.system() {
            namespace.mount(Mount::RoBindTry, Path::new(host), host);
        }
        namespace
    }

    /// A deterministic identity and a files-only resolver, generated into
    /// `etc` and bound read-only over the box's `/etc`: `localhost`
    /// resolves inside a no-network namespace without exposing the host's
    /// resolver or its network namespace, and nothing of the host's own
    /// identity files is copied in.
    pub(super) fn identity(&mut self, etc: &Path) -> std::io::Result<()> {
        generate(etc, Resolver::Files)?;
        self.generated_in(etc, Resolver::Files);
        Ok(())
    }

    /// Bind the generated identity files in `etc` at their places.
    fn generated_in(&mut self, etc: &Path, resolver: Resolver) {
        for (name, _) in generated(resolver) {
            self.mount(Mount::RoBind, &etc.join(name), &identity_target(name));
        }
    }

    /// Bind the host's resolver file where `resolver` asks the host's DNS.
    fn resolving(&mut self, resolver: Resolver) {
        match resolver {
            Resolver::Files => {}
            Resolver::Dns => self.mount_in_place(Mount::RoBindTry, Path::new(RESOLV_CONF)),
        }
    }

    /// Bind each server system source whose canonical path the set does
    /// not already name at that path too, from the same source: an
    /// executable the layout admits where it resolves is there in the box,
    /// and the alias adds no source (D5).
    fn aliased(&mut self) {
        let sources: Vec<&str> = Profile::Server.system().collect();
        for (source, canonical) in aliases_with(&sources, on_host) {
            self.mount(
                Mount::RoBindTry,
                Path::new(source),
                &namespace_path(&canonical),
            );
        }
    }

    /// `flag` making `target` inside the box: a skeleton or a tmpfs.
    fn made(&mut self, flag: &str, target: &str) {
        self.targets.push(PathBuf::from(target));
        self.argv.extend([flag, target].map(String::from));
    }

    /// Mount `host` at `target` inside the box.
    pub(super) fn mount(&mut self, mount: Mount, host: &Path, target: &str) {
        self.paths
            .extend([host.to_path_buf(), PathBuf::from(target)]);
        self.targets.push(PathBuf::from(target));
        let at = self.argv.len();
        self.binds.push(Bind {
            at,
            host: host.to_path_buf(),
        });
        self.argv.extend([
            mount.flag().to_string(),
            host.to_string_lossy().into_owned(),
            target.to_string(),
        ]);
    }

    /// Mount `host` at its own path inside the box.
    pub(super) fn mount_in_place(&mut self, mount: Mount, host: &Path) {
        self.mount(mount, host, &namespace_path(host));
    }

    /// An empty tmpfs over `target`, hiding what lies beneath it.
    pub(super) fn tmpfs(&mut self, target: &str) {
        self.made("--tmpfs", target);
    }

    /// A private directory: an empty tmpfs only its owner may enter,
    /// whatever umask the launcher runs under (#570).
    fn private(&mut self, target: &str) {
        self.argv.extend(["--perms", "0700"].map(String::from));
        self.tmpfs(target);
    }

    /// Mount every host source from its observed handle (`--ro-bind-fd`),
    /// not its name, dropping an optional source with none: the box holds
    /// the object observed, whatever its path names later (MB3). bubblewrap
    /// closes each descriptor it mounts, so a source mounted twice (a
    /// system source and its alias) gets a duplicate, returned for the box
    /// to hold; one that cannot be made leaves identity unprotected.
    pub(super) fn backed(
        &mut self,
        handles: &[(PathBuf, OwnedFd)],
    ) -> Result<Vec<(PathBuf, OwnedFd)>, Refusal> {
        let mut argv = Vec::with_capacity(self.argv.len());
        let (mut from, mut used, mut duplicates) = (0, Vec::new(), Vec::new());
        for bind in std::mem::take(&mut self.binds) {
            argv.extend_from_slice(&self.argv[from..bind.at]);
            let held = handles.iter().find(|(host, _)| *host == bind.host);
            if let Some((_, fd)) = held {
                let mut number = fd.as_raw_fd();
                if used.contains(&number) {
                    let duplicate = fd.try_clone().map_err(|_| Refusal::Identity)?;
                    number = duplicate.as_raw_fd();
                    duplicates.push((bind.host.clone(), duplicate));
                }
                used.push(number);
                let target = self.argv[bind.at + 2].clone();
                argv.extend(["--ro-bind-fd".to_string(), number.to_string(), target]);
            }
            from = bind.at + 3;
        }
        argv.extend_from_slice(&self.argv[from..]);
        self.argv = argv;
        Ok(duplicates)
    }

    /// An overlay mount, as `overlay_argv` serialized it for its writes.
    pub(super) fn overlay(&mut self, argv: Vec<String>) {
        self.argv.extend(argv);
    }

    /// One entry of the box's environment, which `open` cleared.
    pub(super) fn setenv(&mut self, key: &str, value: &str) {
        self.argv.extend(["--setenv", key, value].map(String::from));
    }

    /// The finished argv: enter `workdir` inside the box, then run
    /// `command` there.
    pub(super) fn enter(mut self, workdir: &Path, command: &[String]) -> Vec<String> {
        self.close(workdir, command);
        self.argv
    }

    /// Close the argv in place: enter `workdir`, then run `command`.
    fn close(&mut self, workdir: &Path, command: &[String]) {
        self.argv.extend([
            "--chdir".to_string(),
            namespace_path(workdir),
            "--".to_string(),
        ]);
        self.argv.extend(command.iter().cloned());
    }
}

/// `path` canonical on this host, or none where it does not resolve: the
/// resolver the builder's system checks take, a test planting another.
pub(super) fn on_host(path: &Path) -> Option<PathBuf> {
    std::fs::canonicalize(path).ok()
}

/// Each of `sources` paired with the canonical path `resolve` gives it,
/// where that path is not itself one of `sources`: a source that is no
/// link, or links to another source, needs no alias; an absent one has
/// none.
pub(super) fn aliases_with<'s>(
    sources: &[&'s str],
    resolve: impl Fn(&Path) -> Option<PathBuf>,
) -> Vec<(&'s str, PathBuf)> {
    sources
        .iter()
        .filter_map(|source| {
            let canonical = resolve(Path::new(source))?;
            let named = sources.iter().any(|other| Path::new(other) == canonical);
            (!named).then_some((*source, canonical))
        })
        .collect()
}

/// The identity files every box generates, by name under `/etc`, with
/// their text: the engine's own ids under `runner`, and the resolver
/// `resolver` names.
fn generated(resolver: Resolver) -> [(&'static str, String); 4] {
    let (uid, gid) = ids();
    let hosts = match resolver {
        Resolver::Files => "hosts: files\n",
        Resolver::Dns => "hosts: files dns\n",
    };
    [
        (
            "passwd",
            format!("runner:x:{uid}:{gid}:brokkr hands:{SANDBOX_HOME}:/bin/sh\n"),
        ),
        ("group", format!("runner:x:{gid}:\n")),
        (
            "hosts",
            "127.0.0.1 localhost\n::1 localhost ip6-localhost ip6-loopback\n".to_string(),
        ),
        ("nsswitch.conf", hosts.to_string()),
    ]
}

/// Where the generated identity file `name` lies inside the box.
fn identity_target(name: &str) -> String {
    format!("/etc/{name}")
}

/// Write the generated identity into `etc`.
fn generate(etc: &Path, resolver: Resolver) -> std::io::Result<()> {
    std::fs::create_dir_all(etc)?;
    for (name, text) in generated(resolver) {
        std::fs::write(etc.join(name), text)?;
    }
    Ok(())
}

/// MB3's shared system bin directories: an executable directly in one is
/// a system entry, its own program tree.
const SYSTEM_BINS: [&str; 6] = [
    "/usr/bin",
    "/usr/sbin",
    "/usr/local/bin",
    "/usr/local/sbin",
    "/bin",
    "/sbin",
];

/// The fixed search path a server's bare launch name is found on, and the
/// `PATH` its box sets: never the cwd or an ambient `PATH` (MB3, MB4).
const SERVER_PATH: &str = "/usr/local/bin:/usr/bin:/bin";

/// MB4's fixed server environment beside the [`HANDS_BOX_ENV`] marker
/// every box opens with, in the order it is set.
const SERVER_ENVIRONMENT: [(&str, &str); 7] = [
    ("PATH", SERVER_PATH),
    ("HOME", SANDBOX_HOME),
    ("TMPDIR", "/tmp"),
    ("USER", "runner"),
    ("LOGNAME", "runner"),
    ("LANG", "C.UTF-8"),
    ("LC_ALL", "C.UTF-8"),
];

/// The names of a server box's whole fixed environment, in order: the
/// reserved set no binding name may take (MB4). They are read from the
/// table the builder sets, so the entries set and the names refused are
/// one fact.
pub fn server_environment() -> impl Iterator<Item = &'static str> {
    SERVER_ENVIRONMENT
        .iter()
        .map(|(name, _)| *name)
        .chain([HANDS_BOX_ENV])
}

/// `path` with every symlink resolved, or as spelled where it does not
/// resolve.
fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// A path as spelled and as it resolves: what a reach check compares.
fn spellings(path: &Path) -> [PathBuf; 2] {
    [path.to_path_buf(), canonical(path)]
}

/// Whether `path` lies within `root`, or holds it.
fn overlaps(path: &Path, root: &Path) -> bool {
    path.starts_with(root) | root.starts_with(path)
}

/// A server's program as MB3 resolves it from its launch name alone,
/// without running it or reading its arguments, shebang or loader
/// (U6c4): the file the name names, the executable that file resolves
/// to, and the closed program tree that executable's layout gives.
#[derive(Debug)]
pub struct ServerProgram {
    launch: PathBuf,
    executable: PathBuf,
    tree: Tree,
}

impl ServerProgram {
    /// The program `launch` names, against the trusted canonical host
    /// `home`, or MB3's program-tree cause.
    pub fn resolve(launch: &str, home: &Path) -> Result<ServerProgram, Refusal> {
        let named = located(launch).ok_or(Refusal::ProgramTree)?;
        let executable = std::fs::canonicalize(&named).ok();
        let executable = executable.ok_or(Refusal::ProgramTree)?;
        let tree = layout(&executable, home)?;
        Ok(ServerProgram {
            launch: named,
            executable,
            tree,
        })
    }

    /// The canonical executable the box runs.
    pub fn executable(&self) -> &Path {
        &self.executable
    }

    /// MB3's program tree of the executable.
    pub fn tree(&self) -> &Tree {
        &self.tree
    }
}

/// The file `launch` names: an absolute path as written, or a bare name
/// on [`SERVER_PATH`]; a relative path holding a `/` names nothing. Only
/// an executable regular file, symlinks followed, is named.
fn located(launch: &str) -> Option<PathBuf> {
    let candidates = match (launch.starts_with('/'), launch.contains('/')) {
        (true, _) => vec![PathBuf::from(launch)],
        (false, true) => Vec::new(),
        (false, false) => SERVER_PATH
            .split(':')
            .map(|dir| Path::new(dir).join(launch))
            .collect(),
    };
    let runnable = |path: &PathBuf| {
        path.metadata()
            .is_ok_and(|held| held.is_file() & (held.permissions().mode() & 0o111 != 0))
    };
    candidates.into_iter().find(runnable)
}

/// MB3's closed distinction over a canonical executable: directly in a
/// canonical shared system bin it is a system entry, its own tree;
/// anywhere else its package is its parent, or that parent's parent when
/// the parent is a `bin` or `sbin`, and never `/`, the host HOME or an
/// ancestor of it.
pub(super) fn layout(executable: &Path, home: &Path) -> Result<Tree, Refusal> {
    let parent = executable.parent().ok_or(Refusal::ProgramTree)?;
    let system = SYSTEM_BINS
        .iter()
        .any(|bin| canonical(Path::new(bin)) == parent);
    if system {
        return Ok(Tree::System {});
    }
    let named = |dir: &str| parent.file_name() == Some(OsStr::new(dir));
    let root = match named("bin") | named("sbin") {
        // A directory named `bin` is never `/`, so it has a parent.
        true => parent.parent().unwrap_or(parent),
        false => parent,
    };
    // `home` is absolute, so `/` is among the roots it starts with.
    (!home.starts_with(root))
        .then_some(Tree::Package {
            root: root.to_path_buf(),
        })
        .ok_or(Refusal::ProgramTree)
}

/// A server box's private directories (MB4), in the order they are made.
const PRIVATE: [&str; 2] = [SANDBOX_HOME, "/tmp"];

/// Whether `path` lies in a server box's private directory.
fn in_private(path: &Path) -> bool {
    PRIVATE.iter().any(|dir| path.starts_with(dir))
}

/// Whether `profile`'s system set, each source canonical as `resolve`
/// gives it, holds `path`: a tree inside the server's keeps its own
/// identity and gains no mount, never a broader one.
fn covered(profile: Profile, path: &Path, resolve: &impl Fn(&Path) -> Option<PathBuf>) -> bool {
    profile
        .system()
        .any(|source| resolve(Path::new(source)).is_some_and(|source| path.starts_with(source)))
}

impl Namespace {
    /// A server box up to its program: the projected system set and its
    /// canonical aliases, then the private owner-only tmpfs HOME and
    /// `/tmp`, which start empty: no source may be mounted below either.
    pub(super) fn server(network: bool) -> Namespace {
        let mut namespace = Namespace::open(Profile::Server, network);
        namespace.aliased();
        for private in PRIVATE {
            namespace.private(private);
        }
        namespace
    }

    /// Bind `executable`'s program tree where the server's system set,
    /// its sources canonical as `resolve` gives them, does not hold it,
    /// read-only at its canonical path (MB3, MB4). A system entry is its
    /// one file: on a host whose `/usr/sbin` is no source and no alias's
    /// destination it is bound alone, never its directory. A package
    /// root is refused where it is or holds a path the box already set
    /// up, which would widen the system set, replace the generated
    /// identity's `/etc` or shadow a private directory, where it lies
    /// inside a private directory, which must start empty (MB4), or where
    /// it lies inside a source the server's projection narrowed away
    /// (`/etc/ssl/private`).
    pub(super) fn program_with(
        &mut self,
        tree: &Tree,
        executable: &Path,
        resolve: impl Fn(&Path) -> Option<PathBuf>,
    ) -> Result<(), Refusal> {
        match tree {
            Tree::Package { root } if !covered(Profile::Server, root, &resolve) => {
                let shadows = self.targets.iter().any(|target| target.starts_with(root));
                let narrowed = covered(Profile::Workspace, root, &resolve);
                (!(shadows | narrowed | in_private(root)))
                    .then_some(())
                    .ok_or(Refusal::ProgramTree)?;
                self.mount_in_place(Mount::RoBind, root);
            }
            Tree::System {} if !covered(Profile::Server, executable, &resolve) => {
                self.mount_in_place(Mount::RoBind, executable);
            }
            Tree::Package { .. } | Tree::System {} => {}
        }
        Ok(())
    }
}

/// What a server box is built from beside its program (U6c4): the seat's
/// reach it must stay clear of, the network the dialect's egress projects
/// to, the trusted bootstrap it binds as one file, and the arguments the
/// program runs with.
pub struct ServerProfile<'a> {
    pub reach: &'a Reach,
    pub network: &'a Network,
    pub bootstrap: &'a Path,
    pub arguments: &'a [String],
}

/// One MCP server's box, prepared and never started (U6c4; MB3, MB4): the
/// isolation every box has but no session of its own, the projected
/// system set and its canonical aliases, a fresh private tmpfs HOME and
/// TMPDIR, the program tree where the system set does not already hold
/// it, the bootstrap, the generated identity and resolver, MB4's fixed
/// environment, and the executable entered from `/runtime/home`. No
/// workspace, Git, bundle, overlay, declared hands bind or host HOME is
/// mounted. Its generated identity lives in a private session tree this
/// value holds and removes. Every source is mounted from the handle its
/// observer checked (U6c5a), which the box holds, never plan authority.
#[derive(Debug)]
pub struct ServerBox {
    intent: Namespace,
    _scratch: Session,
    _handles: Vec<(PathBuf, OwnedFd)>,
    _sources: Sources,
}

/// What the observer gives a prepared box: each observed source's handle
/// by the host path that named it, and the facts a plan seals of the set.
type Observed = (Vec<(PathBuf, OwnedFd)>, Sources);

impl ServerBox {
    /// The box `program` runs in under `profile`, or the first of MB3's
    /// causes in order: a package root that would widen or shadow the box,
    /// the executable launched from writable reach, then any source or
    /// destination overlapping any reach root, either way, the temporary
    /// directory the generated identity is written under included. Nothing
    /// is made before reach clears. Then the observer's causes over every
    /// source, in MB3's order with identity setup's own: a bootstrap in a
    /// private directory, or a scratch that cannot hold the identity,
    /// leaves identity unprotected, after any linked program or bootstrap
    /// file. Last, a launcher that cannot mount a descriptor, which off
    /// Linux none can.
    pub fn prepare(
        program: &ServerProgram,
        profile: &ServerProfile<'_>,
    ) -> Result<ServerBox, Refusal> {
        #[cfg(target_os = "linux")]
        let observe = |namespace: &Namespace, made: Option<&Path>| {
            sources::served(namespace, made, program, profile, &sources::Host::live())
        };
        #[cfg(not(target_os = "linux"))]
        let observe = |_: &Namespace, _: Option<&Path>| -> Result<Observed, Refusal> {
            Err(Refusal::Unavailable)
        };
        ServerBox::prepared(program, profile, observe)
    }

    /// [`ServerBox::prepare`] with `host` standing for this host's facts.
    #[cfg(all(test, target_os = "linux"))]
    pub(super) fn prepare_with(
        program: &ServerProgram,
        profile: &ServerProfile<'_>,
        host: &sources::Host<'_>,
    ) -> Result<ServerBox, Refusal> {
        let observe = |namespace: &Namespace, made: Option<&Path>| {
            sources::served(namespace, made, program, profile, host)
        };
        ServerBox::prepared(program, profile, observe)
    }

    /// The box, `observe` reading the namespace and the directory its
    /// identity was generated in, where it could be.
    fn prepared(
        program: &ServerProgram,
        profile: &ServerProfile<'_>,
        observe: impl FnOnce(&Namespace, Option<&Path>) -> Result<Observed, Refusal>,
    ) -> Result<ServerBox, Refusal> {
        let (network, resolver) = match profile.network {
            Network::Isolated => (false, Resolver::Files),
            Network::Shared => (true, Resolver::Dns),
        };
        let mut namespace = Namespace::server(network);
        namespace.program_with(&program.tree, &program.executable, on_host)?;
        namespace.mount_in_place(Mount::RoBind, profile.bootstrap);
        namespace.resolving(resolver);
        // The identity's places are checked now; its sources lie in a
        // tree `Session::create` makes under `temporary` once reach clears.
        let places = generated(resolver).map(|(name, _)| PathBuf::from(identity_target(name)));
        let planned = [&namespace.paths[..], &places[..]].concat();
        let temporary = std::env::temp_dir();
        clear(program, &planned, &temporary, profile.reach)?;
        let identity = identity(profile.bootstrap, resolver);
        let made = identity.as_ref().ok().map(|(_, etc)| etc.clone());
        if let Some(etc) = &made {
            namespace.generated_in(etc, resolver);
        }
        for (key, value) in SERVER_ENVIRONMENT {
            namespace.setenv(key, value);
        }
        // The sources are observed whether or not the identity could be
        // made: a cause MB3 puts before identity, a linked program or
        // bootstrap file, still wins.
        let observed = observe(&namespace, made.as_deref());
        let ((scratch, _), (mut handles, sources)) = match (identity, observed) {
            (Ok(identity), Ok(observed)) => (identity, observed),
            (Err(made), Err(observed)) => return Err(made.min(observed)),
            (Err(cause), Ok(_)) | (Ok(_), Err(cause)) => return Err(cause),
        };
        handles.extend(namespace.backed(&handles)?);
        let command: Vec<String> = [namespace_path(&program.executable)]
            .into_iter()
            .chain(profile.arguments.iter().cloned())
            .collect();
        namespace.close(Path::new(SANDBOX_HOME), &command);
        Ok(ServerBox {
            intent: namespace,
            _scratch: scratch,
            _handles: handles,
            _sources: sources,
        })
    }

    /// Every host source and box destination the box mounts.
    pub fn paths(&self) -> impl Iterator<Item = &Path> {
        self.intent.paths.iter().map(PathBuf::as_path)
    }

    /// The observed source set: its entries, mount records and digest.
    /// Read by tests until admission compares it with the plan's sealed
    /// sources (U6c5b).
    #[cfg(all(test, target_os = "linux"))]
    pub(super) fn sources(&self) -> &Sources {
        &self._sources
    }

    /// Each held handle and the host path that named it. Read by tests
    /// until the launch passes them to bubblewrap.
    #[cfg(all(test, target_os = "linux"))]
    pub(super) fn handles(&self) -> impl Iterator<Item = (&Path, &OwnedFd)> {
        self._handles.iter().map(|(path, fd)| (path.as_path(), fd))
    }

    /// The bubblewrap argv that builds the box. Read by tests until the
    /// launch runs it.
    #[cfg(all(test, target_os = "linux"))]
    pub(super) fn argv(&self) -> &[String] {
        &self.intent.argv
    }
}

/// The private session tree a server box's identity is generated in, and
/// its `/etc`; identity unprotected where the bootstrap lies in a private
/// directory, which must start empty (MB4), or where no scratch can hold
/// the identity.
fn identity(bootstrap: &Path, resolver: Resolver) -> Result<(Session, PathBuf), Refusal> {
    (!in_private(bootstrap))
        .then_some(())
        .ok_or(Refusal::Identity)?;
    let scratch = Session::create("server").map_err(|_| Refusal::Identity)?;
    let etc = scratch.path().join("etc");
    generate(&etc, resolver).ok().ok_or(Refusal::Identity)?;
    Ok((scratch, etc))
}

/// The seat's reach against the box (MB3), each path compared as spelled
/// and as it resolves: neither the launch name's file nor the executable
/// in a writable root, then no path the box mounts within any root or
/// holding one, and no root holding the `temporary` directory the box's
/// session tree will be made in. That tree is made later under a fresh
/// random name, so a root can lie inside it only by naming it first.
fn clear(
    program: &ServerProgram,
    paths: &[PathBuf],
    temporary: &Path,
    reach: &Reach,
) -> Result<(), Refusal> {
    let resolved = |roots: &[PathBuf]| -> Vec<PathBuf> {
        roots.iter().flat_map(|root| spellings(root)).collect()
    };
    let writable = resolved(&reach.writable);
    let launched = [&program.launch, &program.executable]
        .into_iter()
        .flat_map(|path| spellings(path))
        .any(|path| writable.iter().any(|root| path.starts_with(root)));
    (!launched).then_some(()).ok_or(Refusal::LaunchInReach)?;
    let roots = [writable, resolved(&reach.readable)].concat();
    let overlapping = paths
        .iter()
        .flat_map(|path| spellings(path))
        .any(|path| roots.iter().any(|root| overlaps(&path, root)));
    let held = spellings(temporary)
        .iter()
        .any(|dir| roots.iter().any(|root| dir.starts_with(root)));
    (!(overlapping | held))
        .then_some(())
        .ok_or(Refusal::BindOverlapsReach)
}
