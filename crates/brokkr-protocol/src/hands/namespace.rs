//! The namespace a box is built in, and the bubblewrap argv that says so
//! (decision 0065 slice two, U6c3 and U6c4): the isolation flags, the empty
//! root's skeleton, the host system set read-only, the generated identity,
//! and the environment and command that close the argv. What a box mounts
//! beyond that is its profile's: the workspace's workdir, git and declared
//! binds stay with `box_argv`; one MCP server's program tree, bootstrap and
//! private directories are [`ServerBox`]'s.

use std::ffi::OsStr;
use std::os::unix::fs::PermissionsExt;
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
/// so what a profile checks is exactly what it mounts.
pub(super) struct Namespace {
    argv: Vec<String>,
    paths: Vec<PathBuf>,
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
        let skeleton = [
            "--clearenv",
            "--setenv",
            HANDS_BOX_ENV,
            "1",
            "--proc",
            "/proc",
        ];
        argv.extend(
            skeleton
                .into_iter()
                .chain(["--dev", "/dev"])
                .map(String::from),
        );
        for dir in ["/runtime", "/etc", "/home", "/root", "/run", "/usr"] {
            argv.extend(["--dir", dir].map(String::from));
        }
        let mut namespace = Namespace {
            argv,
            paths: Vec::new(),
        };
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
        self.identified(etc, Resolver::Files);
        Ok(())
    }

    /// Bind the generated identity in `etc`, and the host's resolver file
    /// where `resolver` asks the host's DNS.
    fn identified(&mut self, etc: &Path, resolver: Resolver) {
        for (name, _) in generated(resolver) {
            self.mount(Mount::RoBind, &etc.join(name), &format!("/etc/{name}"));
        }
        match resolver {
            Resolver::Files => {}
            Resolver::Dns => self.mount_in_place(Mount::RoBindTry, Path::new(RESOLV_CONF)),
        }
    }

    /// Mount `host` at `target` inside the box.
    pub(super) fn mount(&mut self, mount: Mount, host: &Path, target: &str) {
        self.paths
            .extend([host.to_path_buf(), PathBuf::from(target)]);
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
        self.argv
            .extend(["--tmpfs".to_string(), target.to_string()]);
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
        self.argv.extend([
            "--chdir".to_string(),
            namespace_path(workdir),
            "--".to_string(),
        ]);
        self.argv.extend(command.iter().cloned());
        self.argv
    }
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

/// `holds`, or `refusal`.
fn ensure(holds: bool, refusal: Refusal) -> Result<(), Refusal> {
    match holds {
        true => Ok(()),
        false => Err(refusal),
    }
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
    ensure(!home.starts_with(root), Refusal::ProgramTree)?;
    Ok(Tree::Package {
        root: root.to_path_buf(),
    })
}

/// Whether the server's system set already binds `root`: a package inside
/// it keeps its own identity and gains no mount, never a broader one.
fn covered(root: &Path) -> bool {
    Profile::Server
        .system()
        .any(|source| std::fs::canonicalize(source).is_ok_and(|source| root.starts_with(source)))
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
/// system set, the generated identity and resolver, a fresh private tmpfs
/// HOME and TMPDIR, the program tree where the system set does not already
/// hold it, the bootstrap, MB4's fixed environment, and the executable
/// entered from `/runtime/home`. No workspace, Git, bundle, overlay,
/// declared hands bind or host HOME is mounted. Its generated identity
/// lives in a private session tree this value holds and removes.
#[derive(Debug)]
pub struct ServerBox {
    paths: Vec<PathBuf>,
    argv: Vec<String>,
    _scratch: Session,
}

impl ServerBox {
    /// The box `program` runs in under `profile`, or the first of MB3's
    /// causes in order: the executable launched from writable reach, then
    /// any source or destination overlapping any reach root, either way.
    /// Nothing is written into the scratch before reach is cleared; a
    /// scratch that cannot hold the generated identity leaves the box's
    /// identity unprotected.
    pub fn prepare(
        program: &ServerProgram,
        profile: &ServerProfile<'_>,
    ) -> Result<ServerBox, Refusal> {
        let (network, resolver) = match profile.network {
            Network::Isolated => (false, Resolver::Files),
            Network::Shared => (true, Resolver::Dns),
        };
        let scratch = Session::create("server").ok();
        let scratch = scratch.ok_or(Refusal::Identity)?;
        let etc = scratch.path().join("etc");
        let mut namespace = Namespace::open(Profile::Server, network);
        namespace.identified(&etc, resolver);
        // The private directories go in before the program and the
        // bootstrap, so a source under /tmp lies on top of the private
        // /tmp rather than hidden beneath it.
        namespace.tmpfs(SANDBOX_HOME);
        namespace.tmpfs("/tmp");
        match &program.tree {
            Tree::Package { root } if !covered(root) => {
                namespace.mount_in_place(Mount::RoBind, root)
            }
            Tree::Package { .. } | Tree::System {} => {}
        }
        namespace.mount_in_place(Mount::RoBind, profile.bootstrap);
        clear(program, &namespace.paths, profile.reach)?;
        generate(&etc, resolver).ok().ok_or(Refusal::Identity)?;
        for (key, value) in SERVER_ENVIRONMENT {
            namespace.setenv(key, value);
        }
        let command: Vec<String> = [namespace_path(&program.executable)]
            .into_iter()
            .chain(profile.arguments.iter().cloned())
            .collect();
        let paths = namespace.paths.clone();
        Ok(ServerBox {
            paths,
            argv: namespace.enter(Path::new(SANDBOX_HOME), &command),
            _scratch: scratch,
        })
    }

    /// Every host source and box destination the box mounts.
    pub fn paths(&self) -> impl Iterator<Item = &Path> {
        self.paths.iter().map(PathBuf::as_path)
    }

    /// The bubblewrap argv that builds the box, for the launch to run.
    pub fn argv(&self) -> &[String] {
        &self.argv
    }
}

/// The seat's reach against the box (MB3), each path compared as spelled
/// and as it resolves: neither the launch name's file nor the executable
/// in a writable root, then no path the box mounts within any root or
/// holding one.
fn clear(program: &ServerProgram, paths: &[PathBuf], reach: &Reach) -> Result<(), Refusal> {
    let resolved = |roots: &[PathBuf]| -> Vec<PathBuf> {
        roots.iter().flat_map(|root| spellings(root)).collect()
    };
    let writable = resolved(&reach.writable);
    let launched = [&program.launch, &program.executable]
        .into_iter()
        .flat_map(|path| spellings(path))
        .any(|path| writable.iter().any(|root| path.starts_with(root)));
    ensure(!launched, Refusal::LaunchInReach)?;
    let roots = [writable, resolved(&reach.readable)].concat();
    let overlapping = paths.iter().flat_map(|path| spellings(path)).any(|path| {
        roots
            .iter()
            .any(|root| path.starts_with(root) | root.starts_with(&path))
    });
    ensure(!overlapping, Refusal::BindOverlapsReach)
}
