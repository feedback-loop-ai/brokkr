//! The server box (decision 0065 slice two, U6c4; MB3, MB4): the closed
//! profile one MCP server runs in, its program resolved from the launch
//! name alone, prepared against the seat's reach, and launched on its entry
//! (U6c6b) with a shell standing for the bootstrap.

use std::ffi::OsStr;
#[cfg(target_os = "linux")]
use std::io::Read;
#[cfg(target_os = "linux")]
use std::os::fd::{AsFd, AsRawFd};
use std::os::unix::fs::{MetadataExt, PermissionsExt};

use super::super::*;
use crate::broker::{Network, Reach, Refusal, Tree};

/// What a box whose reach and tree admit it answers: prepared on Linux;
/// elsewhere no Linux box launcher stands (MB3).
pub(super) const ADMITTED: Result<(), Refusal> = match cfg!(target_os = "linux") {
    true => Ok(()),
    false => Err(Refusal::Unavailable),
};

/// Whether this run stands in a seat's hands box, where no box is prepared.
pub(super) fn boxed() -> bool {
    // An unprivileged bubblewrap user namespace maps root-owned host files to the overflow uid, so MB3 rightly refuses their filesystem identity.
    std::env::var_os(HANDS_BOX_ENV).is_some()
}

/// Skip a proof only a prepared box gives: a run that declared boundary
/// evidence fails instead of passing on it.
pub(super) fn skip_in_box() {
    let reason = "a seat's hands box prepares no server box";
    skip_boundary_proof(boundary_evidence_required(), reason);
}

/// Skip a proof only a live box gives where none can stand: in a seat's
/// hands box. A host's root-only system files are no reason (operator
/// ruling 2026-10-07): MB3 admits them, so a live box stands beside them.
/// A run that declared boundary evidence fails instead of passing on it.
pub(super) fn unservable() -> bool {
    let boxed = boxed();
    if boxed {
        skip_in_box();
    }
    boxed
}

/// A canonicalised temporary root with a host HOME inside it. It lies in
/// the build's own directory rather than the temporary directory: a
/// server box mounts no source below its private `/tmp` (MB4).
pub(super) struct Host {
    _dir: tempfile::TempDir,
    pub(super) root: PathBuf,
    pub(super) home: PathBuf,
}

/// The build's own directory, the test binary's `deps` directory's parent.
pub(super) fn build_dir() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    exe.parent().unwrap().parent().unwrap().to_path_buf()
}

impl Host {
    pub(super) fn new() -> Host {
        Host::under(&build_dir())
    }

    /// The root made under `parent` instead.
    pub(super) fn under(parent: &Path) -> Host {
        let dir = tempfile::Builder::new()
            .prefix("brokkr-u6c5a-")
            .tempdir_in(parent)
            .unwrap();
        let root = dir.path().canonicalize().unwrap();
        let home = root.join("home");
        std::fs::create_dir_all(&home).unwrap();
        Host {
            _dir: dir,
            root,
            home,
        }
    }

    pub(super) fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    /// A file at `relative` with `mode`.
    pub(super) fn file(&self, relative: &str, mode: u32) -> PathBuf {
        let path = self.path(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "#!/bin/sh\n").unwrap();
        let mode = std::os::unix::fs::PermissionsExt::from_mode(mode);
        std::fs::set_permissions(&path, mode).unwrap();
        path
    }

    /// An executable at `relative`.
    pub(super) fn plant(&self, relative: &str) -> PathBuf {
        self.file(relative, 0o755)
    }

    /// A symlink at `relative` to `target`.
    pub(super) fn link(&self, relative: &str, target: &Path) -> PathBuf {
        let path = self.path(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(target, &path).unwrap();
        path
    }
}

/// What `launch` resolves to under `home`: the executable, and its package
/// root, or none for a system entry.
fn resolved(launch: &Path, home: &Path) -> Result<(PathBuf, Option<PathBuf>), Refusal> {
    let program = ServerProgram::resolve(launch.to_str().unwrap(), home)?;
    let root = match program.tree() {
        Tree::System {} => None,
        Tree::Package { root } => Some(root.clone()),
    };
    Ok((program.executable().to_path_buf(), root))
}

/// The host's shell named by its path: a system entry wherever `/bin/sh`
/// resolves into a shared system bin, as it does on every supported Linux.
fn shell() -> PathBuf {
    std::fs::canonicalize("/bin/sh").unwrap()
}

/// What a bare `name` resolves to: the first executable the fixed search
/// path holds, which need not be `/bin/sh`'s file where `/bin` and
/// `/usr/bin` are separate directories.
fn found(name: &str) -> PathBuf {
    ["/usr/local/bin", "/usr/bin", "/bin"]
        .iter()
        .map(|dir| Path::new(dir).join(name))
        .find(|path| {
            path.metadata()
                .is_ok_and(|held| held.is_file() & (held.permissions().mode() & 0o111 != 0))
        })
        .map(|path| std::fs::canonicalize(path).unwrap())
        .unwrap()
}

/// Whether some regular file under `root` has a second link, never
/// following one; none for a tree of more than 4,096 entries.
fn links(root: &Path) -> Option<bool> {
    let mut pending = vec![root.to_path_buf()];
    let (mut seen, mut linked) = (0, false);
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(dir).ok()? {
            let path = entry.ok()?.path();
            let held = path.symlink_metadata().ok()?;
            seen += 1;
            linked |= held.is_file() & (std::os::unix::fs::MetadataExt::nlink(&held) > 1);
            if held.is_dir() {
                pending.push(path);
            }
        }
    }
    (seen <= 4096).then_some(linked)
}

/// An installed executable inside the system set that is no system entry:
/// `/usr/lib/<package>/…/<file>` outside any `bin`, a package the system
/// set already holds (e.g. `/usr/lib/apt/methods/http`), whose tree has
/// no multiply-linked file.
pub(super) fn installed() -> (PathBuf, PathBuf) {
    let mut packages: Vec<PathBuf> = std::fs::read_dir("/usr/lib")
        .unwrap()
        .filter_map(|entry| Some(entry.ok()?.path()))
        .collect();
    packages.sort();
    packages
        .iter()
        .filter_map(|dir| std::fs::read_dir(dir).ok())
        .flat_map(|entries| entries.filter_map(|entry| Some(entry.ok()?.path())))
        .filter_map(|path| std::fs::canonicalize(path).ok())
        .find(|path| {
            let parent = path.parent().unwrap();
            let named = parent.file_name().unwrap_or_default();
            let runnable = path
                .metadata()
                .is_ok_and(|held| held.is_file() & (held.permissions().mode() & 0o111 != 0));
            runnable
                && (parent != Path::new("/usr/lib"))
                && parent.starts_with("/usr/lib/")
                && !["bin", "sbin"].map(OsStr::new).contains(&named)
                && links(parent) == Some(false)
        })
        .map(|path| (path.clone(), path.parent().unwrap().to_path_buf()))
        .expect("an executable installed under /usr/lib")
}

#[test]
fn a_system_entry_is_its_own_tree_and_a_package_its_installed_root() {
    let host = Host::new();
    let sh = shell();
    // A bare name is found on the fixed search path; a path names itself;
    // a link into a shared bin resolves to a system entry.
    let link = host.link("links/sh", Path::new("/bin/sh"));
    for (launch, executable) in [
        (Path::new("sh"), found("sh")),
        (Path::new("/bin/sh"), sh.clone()),
        (link.as_path(), sh.clone()),
    ] {
        assert_eq!(
            (launch, resolved(launch, &host.home)),
            (launch, Ok((executable, None)))
        );
    }
    // A package is the executable's parent, or that parent's parent when
    // it is a `bin` or `sbin`, wherever the launch name's link pointed.
    let docs = host.path("opt/docs");
    let entry = host.plant("opt/docs/bin/docs-mcp");
    let into = host.link("links/docs", &entry);
    let cargo = host.home.join(".cargo");
    for (launch, executable, root) in [
        (entry.clone(), entry.clone(), docs.clone()),
        (into, entry.clone(), docs.clone()),
        (
            host.plant("opt/docs/sbin/d"),
            docs.join("sbin/d"),
            docs.clone(),
        ),
        (
            host.plant("opt/tool/server"),
            host.path("opt/tool/server"),
            host.path("opt/tool"),
        ),
        (host.plant("home/.cargo/bin/d"), cargo.join("bin/d"), cargo),
    ] {
        let answer = resolved(&launch, &host.home);
        assert_eq!((&launch, answer), (&launch, Ok((executable, Some(root)))));
    }
    // A package the system set already holds keeps its own root.
    let (executable, root) = installed();
    assert_eq!(
        resolved(&executable, &host.home),
        Ok((executable.clone(), Some(root)))
    );
}

#[test]
fn a_launch_that_names_no_installed_file_or_widens_its_root_refuses() {
    let host = Host::new();
    host.plant("opt/docs/bin/docs-mcp");
    // A relative path holding a `/` names nothing, even one that would
    // reach a real system file from any cwd; a bare name is never looked
    // for beside the cwd or on an ambient PATH.
    let climbing = format!("{}bin/sh", "../".repeat(32));
    for launch in [
        "bin/docs-mcp",
        "./docs-mcp",
        climbing.as_str(),
        "brokkr-u6c4-not-installed",
        "",
    ] {
        let answer = ServerProgram::resolve(launch, &host.home).map(|_| ());
        assert_eq!((launch, answer), (launch, Err(Refusal::ProgramTree)));
    }
    // A missing file, a directory, a file no one may execute, and a root
    // that is the host HOME or holds it.
    for launch in [
        host.path("opt/none/bin/x"),
        host.path("opt/docs/bin"),
        host.file("opt/plain/bin/p", 0o644),
        host.plant("home/d"),
        host.plant("bin/d"),
    ] {
        let answer = resolved(&launch, &host.home);
        assert_eq!((&launch, answer), (&launch, Err(Refusal::ProgramTree)));
    }
    // `/` is never a package root.
    let at_root = namespace::layout(Path::new("/docs-mcp"), &host.home).map(|_| ());
    assert_eq!(at_root, Err(Refusal::ProgramTree));
}

/// The server's system set: the shared table with `/etc/ssl` narrowed to
/// `/etc/ssl/certs`, spelled here and not read from the builder.
pub(super) fn server_system() -> Vec<&'static str> {
    HOST_TOOLCHAIN_BINDS
        .iter()
        .map(|host| match *host {
            "/etc/ssl" => "/etc/ssl/certs",
            other => other,
        })
        .collect()
}

#[test]
fn a_system_source_linked_outside_the_set_is_bound_at_its_canonical_path_too() {
    let sources = [
        "/usr/bin",
        "/usr/lib",
        "/bin",
        "/sbin",
        "/lib64",
        "/etc/ssl/certs",
    ];
    let resolve = |source: &Path| match source.to_str().unwrap() {
        "/bin" => Some(PathBuf::from("/usr/bin")),
        "/sbin" => Some(PathBuf::from("/usr/sbin")),
        "/lib64" => None,
        other => Some(PathBuf::from(other)),
    };
    let aliases = namespace::aliases_with(&sources, resolve);
    assert_eq!(aliases, [("/sbin", PathBuf::from("/usr/sbin"))]);
}

/// The descriptor number `server` mounts `host` from the `nth` time, if it
/// holds one: bubblewrap closes each descriptor it mounts, so a source
/// mounted again takes its own duplicate.
#[cfg(target_os = "linux")]
fn nth_fd(server: &ServerBox, host: &Path, nth: usize) -> Option<String> {
    let mut held = server.handles().filter(|(path, _)| *path == host);
    held.nth(nth).map(|(_, fd)| fd.as_raw_fd().to_string())
}

/// The descriptor number `server` first mounts `host` from.
#[cfg(target_os = "linux")]
fn fd_of(server: &ServerBox, host: &Path) -> Option<String> {
    nth_fd(server, host, 0)
}

/// The server box's argv up to its entry for `server`'s handles, its
/// generated identity in `etc` and the varying pieces: the network flag,
/// whether the host's resolver is bound and the tree bound. Every source is
/// mounted from the handle the box holds for it, and an absent optional one
/// not at all; each system source linked outside the set is bound at its
/// canonical path too, as this host resolves it; both private directories
/// are owner-only. The program is no part of it: the box's entry is the
/// bootstrap's, which the launch closes it on (U6c6b).
#[cfg(target_os = "linux")]
fn server_argv(server: &ServerBox, net: &str, dns: bool, tree: Option<&Path>) -> Vec<String> {
    let mut argv: Vec<String> = format!(
        "bwrap --die-with-parent --unshare-pid --unshare-ipc --unshare-uts \
         --unshare-cgroup-try --cap-drop ALL {net} --clearenv --setenv {HANDS_BOX_ENV} 1 \
         --proc /proc --dev /dev --dir /runtime --dir /etc --dir /home --dir /root --dir /run \
         --dir /usr"
    )
    .split_whitespace()
    .map(String::from)
    .collect();
    let mut mounted: Vec<PathBuf> = Vec::new();
    let mut mount = |argv: &mut Vec<String>, host: &Path, target: &str| {
        let nth = mounted.iter().filter(|done| *done == host).count();
        mounted.push(host.to_path_buf());
        if let Some(fd) = nth_fd(server, host, nth) {
            argv.extend(["--ro-bind-fd".to_string(), fd, target.to_string()]);
        }
    };
    let system = server_system();
    for host in &system {
        mount(&mut argv, Path::new(host), host);
    }
    let canonical = |source: &Path| std::fs::canonicalize(source).ok();
    for (host, alias) in namespace::aliases_with(&system, canonical) {
        mount(&mut argv, Path::new(host), &alias.display().to_string());
    }
    let private = format!("--perms 0700 --tmpfs {SANDBOX_HOME} --perms 0700 --tmpfs /tmp");
    argv.extend(private.split_whitespace().map(String::from));
    let bootstrap = std::env::current_exe().unwrap();
    let resolver = dns.then_some(Path::new("/etc/resolv.conf"));
    for host in tree
        .into_iter()
        .chain([bootstrap.as_path()])
        .chain(resolver)
    {
        mount(&mut argv, host, &host.display().to_string());
    }
    let etc = identity_dir(server);
    for name in ["passwd", "group", "hosts", "nsswitch.conf"] {
        mount(&mut argv, &etc.join(name), &format!("/etc/{name}"));
    }
    argv.extend(
        format!(
            "--setenv PATH /usr/local/bin:/usr/bin:/bin --setenv HOME {SANDBOX_HOME} \
             --setenv TMPDIR /tmp --setenv USER runner --setenv LOGNAME runner \
             --setenv LANG C.UTF-8 --setenv LC_ALL C.UTF-8"
        )
        .split_whitespace()
        .map(String::from),
    );
    argv
}

/// Where the box's generated identity lies: the directory of the passwd
/// file it holds a handle on.
#[cfg(target_os = "linux")]
fn identity_dir(server: &ServerBox) -> PathBuf {
    let scratch = std::env::temp_dir().canonicalize().unwrap();
    let (passwd, _) = server
        .handles()
        .find(|(path, _)| path.starts_with(&scratch) & path.ends_with("etc/passwd"))
        .unwrap();
    passwd.parent().unwrap().to_path_buf()
}

/// Every handle `server` holds is on the object its path names now, every
/// required source has one, and an optional system source has one exactly
/// where it is present on this host.
#[cfg(target_os = "linux")]
fn held_where_present(server: &ServerBox, required: &[&Path]) {
    let identity = |held: &std::fs::Metadata| (held.dev(), held.ino());
    for (path, fd) in server.handles() {
        let opened = std::fs::File::from(fd.try_clone().unwrap())
            .metadata()
            .unwrap();
        let named = std::fs::metadata(path).unwrap();
        assert_eq!((path, identity(&opened)), (path, identity(&named)));
    }
    for path in required {
        assert!(fd_of(server, path).is_some(), "{}", path.display());
    }
    for host in server_system() {
        let present = Path::new(host).exists();
        assert_eq!(
            (host, fd_of(server, Path::new(host)).is_some()),
            (host, present)
        );
    }
    // bubblewrap closes each descriptor it mounts: none is named twice.
    let argv = server.argv();
    let numbers: Vec<&String> = argv
        .windows(2)
        .filter(|pair| pair[0] == "--ro-bind-fd")
        .map(|pair| &pair[1])
        .collect();
    let unique: std::collections::BTreeSet<&&String> = numbers.iter().collect();
    assert_eq!(unique.len(), numbers.len(), "{numbers:?}");
}

/// The box `program` is prepared in under `reach` and `network`, the test
/// binary its bootstrap; on Linux, its writers confined.
fn prepared(
    program: &ServerProgram,
    reach: &Reach,
    network: &Network,
) -> Result<ServerBox, Refusal> {
    let bootstrap = std::env::current_exe().unwrap();
    let profile = ServerProfile {
        reach,
        network,
        bootstrap: &bootstrap,
        writers: &[],
    };
    #[cfg(target_os = "linux")]
    return ServerBox::prepare_with(program, &profile, &super::sources::confined());
    #[cfg(not(target_os = "linux"))]
    ServerBox::prepare(program, &profile)
}

fn reach(writable: &[PathBuf], readable: &[PathBuf]) -> Reach {
    Reach {
        writable: writable.to_vec(),
        readable: readable.to_vec(),
    }
}

#[cfg(target_os = "linux")]
#[test]
fn the_server_box_holds_the_projected_system_set_and_its_own_private_paths() {
    if unservable() {
        return;
    }
    let host = Host::new();
    let bootstrap = std::env::current_exe().unwrap();
    let seat = reach(&[host.path("work")], &[host.path("cache")]);
    // A dedicated package, isolated: its root bound, files-only names.
    let entry = host.plant("opt/docs/bin/docs-mcp");
    let docs = ServerProgram::resolve(entry.to_str().unwrap(), &host.home).unwrap();
    let server = prepared(&docs, &seat, &Network::Isolated).unwrap();
    let etc = identity_dir(&server);
    let tree = host.path("opt/docs");
    let isolated_argv = server.argv().to_vec();
    let expected = server_argv(&server, "--unshare-net", false, Some(&tree));
    assert_eq!(isolated_argv, expected);
    let required = [
        tree.as_path(),
        &bootstrap,
        &etc.join("passwd"),
        &etc.join("hosts"),
    ];
    held_where_present(&server, &required);
    let (uid, gid) = ids();
    let generated = |name: &str| std::fs::read_to_string(etc.join(name)).unwrap();
    assert_eq!(
        generated("passwd"),
        format!("runner:x:{uid}:{gid}:brokkr hands:{SANDBOX_HOME}:/bin/sh\n")
    );
    assert_eq!(generated("nsswitch.conf"), "hosts: files\n");
    // The identity lies in the box's own session tree, gone with the box.
    let scratch = etc.parent().unwrap();
    let named = scratch.file_name().unwrap().to_string_lossy().into_owned();
    assert!(named.starts_with("brokkr-hands-server-"), "{named}");
    drop(server);
    assert!(!scratch.exists());

    // A system entry and a package the system set holds, on the shared
    // network: neither gains a mount, and DNS reads the host's resolver.
    let sh = ServerProgram::resolve("sh", &host.home).unwrap();
    let (executable, _) = installed();
    let installed = ServerProgram::resolve(executable.to_str().unwrap(), &host.home).unwrap();
    for program in [&sh, &installed] {
        let server = prepared(program, &seat, &Network::Shared).unwrap();
        let etc = identity_dir(&server);
        assert_eq!(server.argv(), server_argv(&server, "", true, None));
        held_where_present(&server, &[&bootstrap]);
        let resolver = std::fs::read_to_string(etc.join("nsswitch.conf")).unwrap();
        assert_eq!(resolver, "hosts: files dns\n");
    }
    // No session of its own, none of the workspace's writable or overlay
    // mounts, and nothing of /etc/ssl but its certificates.
    let flags = [
        "--new-session",
        "--bind",
        "--bind-try",
        "--overlay-src",
        "/etc/ssl",
    ];
    let found = |part: &String| flags.contains(&part.as_str());
    assert_eq!(isolated_argv.iter().find(|part| found(part)), None);
}

#[cfg(target_os = "linux")]
#[test]
fn the_fixed_environment_is_mb4s_eight_entries_and_its_names_are_the_reserved_set() {
    let names: Vec<&str> = server_environment().collect();
    assert_eq!(
        names,
        [
            "PATH",
            "HOME",
            "TMPDIR",
            "USER",
            "LOGNAME",
            "LANG",
            "LC_ALL",
            HANDS_BOX_ENV
        ]
    );
    if unservable() {
        return;
    }
    let host = Host::new();
    let sh = ServerProgram::resolve("sh", &host.home).unwrap();
    let server = prepared(&sh, &reach(&[], &[]), &Network::Isolated).unwrap();
    let set: Vec<(&str, &str)> = server
        .argv()
        .windows(3)
        .filter(|entry| entry[0] == "--setenv")
        .map(|entry| (entry[1].as_str(), entry[2].as_str()))
        .collect();
    assert_eq!(
        set,
        [
            (HANDS_BOX_ENV, "1"),
            ("PATH", "/usr/local/bin:/usr/bin:/bin"),
            ("HOME", SANDBOX_HOME),
            ("TMPDIR", "/tmp"),
            ("USER", "runner"),
            ("LOGNAME", "runner"),
            ("LANG", "C.UTF-8"),
            ("LC_ALL", "C.UTF-8"),
        ]
    );
}

#[test]
fn the_server_box_stays_clear_of_every_reach_root_either_way() {
    let host = Host::new();
    let entry = host.plant("opt/docs/bin/docs-mcp");
    let docs = ServerProgram::resolve(entry.to_str().unwrap(), &host.home).unwrap();
    let bootstrap = std::env::current_exe().unwrap();
    let answer = |program: &ServerProgram, seat: Reach| {
        prepared(program, &seat, &Network::Shared).map(|_| ())
    };
    const LAUNCHED: Result<(), Refusal> = Err(Refusal::LaunchInReach);
    const OVERLAPPING: Result<(), Refusal> = Err(Refusal::BindOverlapsReach);
    let alias = host.link("alias", &host.path("opt"));
    let tmp = std::fs::canonicalize(std::env::temp_dir()).unwrap();
    for (writable, readable, expected) in [
        // The executable in writable reach, before any overlap.
        (vec![host.path("opt/docs/bin")], vec![], LAUNCHED),
        (vec![host.path("opt/docs")], vec![], LAUNCHED),
        // The package in a readable (ro or overlay) root, or holding a
        // writable one; a root holding it, by its spelling or its link.
        (vec![], vec![host.path("opt/docs")], OVERLAPPING),
        (vec![host.path("opt/docs/share")], vec![], OVERLAPPING),
        (vec![], vec![host.path("opt")], OVERLAPPING),
        (vec![], vec![alias], OVERLAPPING),
        // An explicit bind into a system source, or one holding it.
        (
            vec![],
            vec![PathBuf::from("/usr/lib/u6c4-tool")],
            OVERLAPPING,
        ),
        (vec![PathBuf::from("/etc")], vec![], OVERLAPPING),
        // The resolver and the generated identity's places, checked
        // before the identity is written.
        (vec![], vec![PathBuf::from("/etc/resolv.conf")], OVERLAPPING),
        (vec![PathBuf::from("/etc/group")], vec![], OVERLAPPING),
        // The bootstrap, either way.
        (vec![], vec![bootstrap.clone()], OVERLAPPING),
        (
            vec![bootstrap.parent().unwrap().to_path_buf()],
            vec![],
            OVERLAPPING,
        ),
        // Disjoint reach admits the box.
        (vec![host.path("work")], vec![host.path("cache")], ADMITTED),
    ] {
        if expected == ADMITTED && unservable() {
            continue;
        }
        let case = format!("{writable:?} {readable:?}");
        let got = answer(&docs, reach(&writable, &readable));
        assert_eq!((&case, got), (&case, expected));
    }
    // A launch name in writable reach, though it resolves outside it.
    let link = host.link("work/docs", &entry);
    let linked = ServerProgram::resolve(link.to_str().unwrap(), &host.home).unwrap();
    assert_eq!(answer(&linked, reach(&[host.path("work")], &[])), LAUNCHED);
    // The generated identity is a source too: a root holding the scratch.
    let sh = ServerProgram::resolve("sh", &host.home).unwrap();
    assert_eq!(answer(&sh, reach(&[], &[tmp])), OVERLAPPING);
}

#[test]
fn a_package_root_that_widens_or_shadows_the_box_refuses() {
    // A root that is or holds a path the box set up: the system set or
    // its alias, the generated identity's `/etc`, a private directory or
    // the skeleton; or one inside a private directory, which would make
    // the private TMPDIR or HOME start populated (MB4).
    let mut refused: Vec<PathBuf> = ["/usr", "/etc", "/etc/ssl", "/tmp", "/runtime", "/run"]
        .into_iter()
        .chain(["/proc", "/dev", "/tmp/docs", "/runtime/home/docs"])
        .map(PathBuf::from)
        .collect();
    // A root inside the source the projection narrowed away.
    refused.extend(std::fs::canonicalize("/etc/ssl").map(|ssl| ssl.join("private")));
    let unbound = closed(Namespace::server(false));
    for root in refused {
        let mut namespace = Namespace::server(false);
        let tree = Tree::Package { root: root.clone() };
        let answer = namespace.program_with(&tree, &root.join("bin/x"), namespace::on_host);
        assert_eq!((&root, answer), (&root, Err(Refusal::ProgramTree)));
        assert_eq!(closed(namespace), unbound);
    }
    // A dedicated root, and one the system set holds, which gains no
    // mount.
    for (root, bound) in [("/opt/docs", true), ("/usr/lib/docs", false)] {
        let tree = Tree::Package { root: root.into() };
        let executable = Path::new(root).join("bin/x");
        let answer = bound_tree(&tree, &executable, namespace::on_host);
        assert_eq!((root, answer), (root, Ok(bound.then_some(root.into()))));
    }
}

/// A server box's argv closed on `/` with no command.
fn closed(namespace: Namespace) -> Vec<String> {
    namespace.enter(Path::new("/"), &[])
}

#[test]
fn a_source_with_no_observed_handle_is_not_mounted() {
    // An optional source the observer found absent holds no handle: the
    // box mounts nothing in its place, never its path.
    let mut namespace = Namespace::server(false);
    assert_eq!(namespace.backed(&[]).unwrap().len(), 0);
    let argv = closed(namespace);
    let mounted = argv.iter().filter(|part| part.starts_with("--ro-bind"));
    assert_eq!(mounted.collect::<Vec<_>>(), Vec::<&String>::new());
    // The rest of the box stands: both private directories are still made.
    let made = argv.iter().filter(|part| *part == "--tmpfs").count();
    assert_eq!(made, 2);
}

/// What `tree` of `executable` adds to a networked server box, its system
/// sources resolved by `resolve`: the one path it binds read-only in place,
/// just before the closing `--chdir`, or none.
fn bound_tree(
    tree: &Tree,
    executable: &Path,
    resolve: fn(&Path) -> Option<PathBuf>,
) -> Result<Option<PathBuf>, Refusal> {
    let mut namespace = Namespace::server(true);
    namespace.program_with(tree, executable, resolve)?;
    let base = closed(Namespace::server(true));
    let argv = closed(namespace);
    let at = base.len() - 3;
    assert_eq!(
        (&argv[..at], &argv[argv.len() - 3..]),
        (&base[..at], &base[at..])
    );
    match &argv[at..argv.len() - 3] {
        [] => Ok(None),
        [flag, host, target] if flag == "--ro-bind" && host == target => {
            Ok(Some(PathBuf::from(host)))
        }
        added => panic!("unexpected mounts {added:?}"),
    }
}

#[test]
fn a_system_entry_the_bound_set_does_not_hold_is_bound_as_its_one_file() {
    // Split `/usr`: every source is its own directory, so `/usr/sbin` is
    // neither a source nor an alias's destination. Merged, `/sbin`'s alias
    // is `/usr/sbin`.
    let split: fn(&Path) -> Option<PathBuf> = |source| Some(source.to_path_buf());
    let merged: fn(&Path) -> Option<PathBuf> = |source| match source.to_str().unwrap() {
        "/bin" => Some(PathBuf::from("/usr/bin")),
        "/sbin" => Some(PathBuf::from("/usr/sbin")),
        other => Some(PathBuf::from(other)),
    };
    let system = Tree::System {};
    for (executable, resolve, bound) in [
        ("/usr/sbin/u6c4-entry", split, true),
        ("/usr/sbin/u6c4-entry", merged, false),
        ("/sbin/u6c4-entry", split, false),
        ("/usr/bin/u6c4-entry", split, false),
        ("/usr/local/sbin/u6c4-entry", split, false),
    ] {
        let answer = bound_tree(&system, Path::new(executable), resolve);
        let expected = Ok(bound.then_some(PathBuf::from(executable)));
        assert_eq!((executable, answer), (executable, expected));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn a_server_box_stands_without_the_seats_or_the_hosts_private_paths() {
    let required = boundary_evidence_required();
    if !super::can_create_namespace() {
        skip_boundary_proof(required, "no namespace can be built here");
        return;
    }
    if unservable() {
        return;
    }
    let host = Host::new();
    let work = host.path("work");
    std::fs::create_dir_all(&work).unwrap();
    let home = std::env::var("HOME").unwrap_or_default();
    let private = Path::new("/etc/ssl/private");
    let certs = match Path::new("/etc/ssl/certs").is_dir() {
        true => "[ -d /etc/ssl/certs ] || exit 17;",
        false => "",
    };
    // Each system source linked outside the set is there at its canonical
    // path, where the layout finds its executables.
    let aliases = namespace::aliases_with(&server_system(), |source| {
        std::fs::canonicalize(source).ok()
    });
    let aliased: String = aliases
        .iter()
        .map(|(_, alias)| format!("[ -d '{}' ] || exit 19; ", alias.display()))
        .collect();
    let script = format!(
        "exec 2>\"/proc/self/fd/$2\"; [ \"$(pwd)\" = {SANDBOX_HOME} ] || exit 10; \
         [ -z \"$(ls -A {SANDBOX_HOME})\" ] && [ -z \"$(ls -A /tmp)\" ] || exit 11; \
         [ \"$(stat -c %a {SANDBOX_HOME}):$(stat -c %a /tmp)\" = 700:700 ] || exit 20; \
         touch {SANDBOX_HOME}/a /tmp/b && [ ! -e /tmp/a ] || exit 12; \
         [ ! -e '{w}' ] && [ ! -e '{home}' ] || exit 13; \
         [ ! -e {p} ] || exit 14; {certs} \
         [ ! -e /etc/resolv.conf ] || exit 15; \
         [ \"$(cat /etc/nsswitch.conf)\" = 'hosts: files' ] || exit 16; \
         [ \"$PATH:$TMPDIR:$USER:${HANDS_BOX_ENV}\" = /usr/local/bin:/usr/bin:/bin:/tmp:runner:1 ] \
         || exit 18; {aliased} \
         [ \"$(tr '\\0' '\\n' < /proc/$$/environ | cut -d= -f1 | sort | tr '\\n' ' ')\" = \
         'BROKKR_HANDS_BOX HOME LANG LC_ALL LOGNAME PATH TMPDIR USER ' ] || exit 21; \
         exec 2>/dev/null; printf ready > \"/proc/self/fd/$2\" && exec sleep 600",
        w = work.display(),
        p = private.display(),
    );
    let sh = ServerProgram::resolve("sh", &host.home).unwrap();
    let bootstrap = shell();
    let seat = reach(&[work], &[]);
    let profile = ServerProfile {
        reach: &seat,
        network: &Network::Isolated,
        bootstrap: &bootstrap,
        writers: &[],
    };
    let mut server = ServerBox::prepare_with(&sh, &profile, &super::sources::confined()).unwrap();
    // The scenario is a private TLS sibling that exists and cannot be
    // read: prepared beside it, the box neither read nor mounted it. A
    // host without one cannot show that, so the proof skips there, which
    // fails wherever boundary evidence is required. (Planting one in an
    // outer namespace needs a nested user namespace, which Ubuntu's
    // unprivileged bwrap profile refuses.)
    let unreadable = private.is_dir() & std::fs::read_dir(private).is_err();
    if !unreadable {
        skip_boundary_proof(required, "/etc/ssl/private is no unreadable directory here");
        return;
    }
    // The box launched on its entry (U6c6b), the shell standing for its
    // bootstrap: it reports on the ready pipe once every check held, then
    // waits as `sleep`, holding what it was given.
    let entry = server.entry(std::process::id()).unwrap();
    let handles: Vec<_> = server.handles().map(|(_, fd)| fd.as_fd()).collect();
    let given = std::cell::Cell::new((0, 0));
    let command = |control: i32, ready: i32| {
        given.set((control, ready));
        let numbers = [control, ready].map(|fd| fd.to_string());
        let args = ["-c", &script, "sh", &numbers[0], &numbers[1]];
        args.map(String::from).to_vec()
    };
    let mut launched = ServerBox::launch(&entry, &handles, command, super::entry::soon()).unwrap();
    let mut info = String::new();
    launched.info.read_to_string(&mut info).unwrap();
    let mut ready = [0; 5];
    if launched.ready.read_exact(&mut ready).is_err() {
        let mut rest = String::new();
        launched.ready.read_to_string(&mut rest).ok();
        panic!(
            "the box refused: {:?} {ready:?} {rest}",
            launched.child.wait()
        );
    }
    assert_eq!(&ready, b"ready");
    // bubblewrap named the box's first process, the launcher's own child,
    // and the waiting process it started holds its standard streams on
    // nothing, and its two control pipes at the very numbers it was given,
    // and no other descriptor: no handle, no info pipe and nothing else of
    // this process.
    let first: serde_json::Value = serde_json::from_str(&info).unwrap();
    let first = first["child-pid"].as_u64().unwrap();
    assert_eq!(parent_of(first), Some(u64::from(launched.child.id())));
    let waiting = waiting_child(first);
    let pipe = |fd: &dyn AsFd| format!("pipe:[{}]", rustix::fs::fstat(fd).unwrap().st_ino);
    let (control, ready) = given.get();
    let null = || "/dev/null".to_string();
    let mut expected = vec![(0, null()), (1, null()), (2, null())];
    expected.push((control.unsigned_abs(), pipe(&launched.control)));
    expected.push((ready.unsigned_abs(), pipe(&launched.ready)));
    expected.sort();
    assert_eq!(descriptors(waiting), expected);
    // Settled when dropped, this process living on: the box's waiting
    // process is gone, and its generated identity with the entry.
    drop(launched);
    assert!(ended(waiting));
    let scratch = scratch_of(&entry);
    assert!(scratch.is_dir());
    drop(entry);
    assert!(!scratch.exists());
}

/// Whether `pid` is gone, or only a zombie, within five seconds.
#[cfg(target_os = "linux")]
fn ended(pid: u64) -> bool {
    let until = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while std::time::Instant::now() < until {
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).unwrap_or_default();
        let state = stat.rsplit_once(") ").map(|(_, rest)| &rest[..1]);
        if matches!(state, None | Some("Z")) {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    false
}

/// The parent of `pid`, from `/proc/<pid>/stat`.
#[cfg(target_os = "linux")]
fn parent_of(pid: u64) -> Option<u64> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    stat.rsplit_once(')')?
        .1
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()
}

/// The one child of `pid` once it waits as `sleep`, within ten seconds.
#[cfg(target_os = "linux")]
fn waiting_child(pid: u64) -> u64 {
    let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while std::time::Instant::now() < until {
        let children = std::fs::read_to_string(format!("/proc/{pid}/task/{pid}/children"));
        let children: Vec<u64> = children
            .unwrap_or_default()
            .split_whitespace()
            .map(|child| child.parse().unwrap())
            .collect();
        let comm = |child: u64| std::fs::read_to_string(format!("/proc/{child}/comm"));
        if let [child] = children[..] {
            if comm(child).is_ok_and(|comm| comm == "sleep\n") {
                return child;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    panic!("no waiting child of {pid}");
}

/// Each descriptor `pid` holds, by number, and what `/proc` links it to.
#[cfg(target_os = "linux")]
fn descriptors(pid: u64) -> Vec<(u32, String)> {
    let dir = std::fs::read_dir(format!("/proc/{pid}/fd")).unwrap();
    let mut held: Vec<(u32, String)> = dir
        .map(|entry| {
            let entry = entry.unwrap();
            let link = std::fs::read_link(entry.path()).unwrap();
            let number = entry.file_name().to_str().unwrap().parse().unwrap();
            (number, link.to_str().unwrap().to_string())
        })
        .collect();
    held.sort();
    held
}

/// The generated identity's tree `entry` hands over.
#[cfg(target_os = "linux")]
pub(super) fn scratch_of(entry: &ServerEntry) -> PathBuf {
    let entry = serde_json::to_value(entry).unwrap();
    serde_json::from_value(entry["identity"].clone()).unwrap()
}
