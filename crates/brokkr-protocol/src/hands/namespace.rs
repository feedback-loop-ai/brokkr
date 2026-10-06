//! The namespace a box is built in, and the bubblewrap argv that says so
//! (decision 0065 slice two, U6c3 and U6c4): the isolation flags, the empty
//! root's skeleton, the host system set read-only, the generated identity,
//! and the environment and command that close the argv. What a box mounts
//! beyond that is its profile's: the workspace's workdir, git and declared
//! binds stay with `box_argv`; one MCP server's program tree, bootstrap and
//! private directories are [`ServerBox`]'s.

use std::ffi::OsStr;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Path, PathBuf};

use super::{ids, Session, HANDS_BOX_ENV, HOST_TOOLCHAIN_BINDS, SANDBOX_HOME};
use crate::broker::{Network, Reach, Refusal, Tree};

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
/// box sets up, mounted or made, so nothing later lands over one.
#[derive(Debug)]
pub(super) struct Namespace {
    argv: Vec<String>,
    paths: Vec<PathBuf>,
    targets: Vec<PathBuf>,
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

/// Make one of the box's private directories, owner-only whatever the
/// caller's umask (#570). A `create_dir_all` directory takes the umask's
/// mode: under 002, Ubuntu's default, the box's `/tmp` and its home would
/// be group-writable 775, and the capability broker's ancestry guard
/// refuses every plan whose path walks a group-writable directory. The
/// mode goes to mkdir(2) itself, which no umask can widen.
pub(super) fn private_dir(path: &Path) -> std::io::Result<()> {
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)
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
    /// canonical aliases, then the private tmpfs HOME and `/tmp`. They go
    /// in before the program and the bootstrap, so a source under `/tmp`
    /// lies on top of the private `/tmp` rather than hidden beneath it.
    pub(super) fn server(network: bool) -> Namespace {
        let mut namespace = Namespace::open(Profile::Server, network);
        namespace.aliased();
        namespace.tmpfs(SANDBOX_HOME);
        namespace.tmpfs("/tmp");
        namespace
    }

    /// Bind `executable`'s program tree where the server's system set,
    /// its sources canonical as `resolve` gives them, does not hold it,
    /// read-only at its canonical path (MB3, MB4). A system entry is its
    /// one file: on a host whose `/usr/sbin` is no source and no alias's
    /// destination it is bound alone, never its directory. A package
    /// root is refused where it is or holds a path the box already set
    /// up, which would widen the system set, replace the generated
    /// identity's `/etc` or shadow a private directory, or where it lies
    /// inside a source the server's projection narrowed away
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
                (!(shadows | narrowed))
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
/// value holds and removes.
#[derive(Debug)]
pub struct ServerBox {
    intent: Namespace,
    _scratch: Session,
}

impl ServerBox {
    /// The box `program` runs in under `profile`, or the first of MB3's
    /// causes in order: a package root that would widen or shadow the box,
    /// the executable launched from writable reach, then any source or
    /// destination overlapping any reach root, either way, the temporary
    /// directory the generated identity is written under included. Nothing
    /// is created or written before reach is cleared; a scratch that
    /// cannot then hold the generated identity leaves the box's identity
    /// unprotected.
    pub fn prepare(
        program: &ServerProgram,
        profile: &ServerProfile<'_>,
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
        let scratch = Session::create("server").ok();
        let scratch = scratch.ok_or(Refusal::Identity)?;
        let etc = scratch.path().join("etc");
        generate(&etc, resolver).ok().ok_or(Refusal::Identity)?;
        namespace.generated_in(&etc, resolver);
        for (key, value) in SERVER_ENVIRONMENT {
            namespace.setenv(key, value);
        }
        let command: Vec<String> = [namespace_path(&program.executable)]
            .into_iter()
            .chain(profile.arguments.iter().cloned())
            .collect();
        namespace.close(Path::new(SANDBOX_HOME), &command);
        Ok(ServerBox {
            intent: namespace,
            _scratch: scratch,
        })
    }

    /// Every host source and box destination the box mounts.
    pub fn paths(&self) -> impl Iterator<Item = &Path> {
        self.intent.paths.iter().map(PathBuf::as_path)
    }

    /// The bubblewrap argv that builds the box. Read by tests until the
    /// launch runs it.
    #[cfg(test)]
    pub(super) fn argv(&self) -> &[String] {
        &self.intent.argv
    }
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
    let overlapping = paths.iter().flat_map(|path| spellings(path)).any(|path| {
        roots
            .iter()
            .any(|root| path.starts_with(root) | root.starts_with(&path))
    });
    let held = spellings(temporary)
        .iter()
        .any(|dir| roots.iter().any(|root| dir.starts_with(root)));
    (!(overlapping | held))
        .then_some(())
        .ok_or(Refusal::BindOverlapsReach)
}
