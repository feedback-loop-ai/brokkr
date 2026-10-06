//! The server box (decision 0065 slice two, U6c4; MB3, MB4): the closed
//! profile one MCP server runs in, its program resolved from the launch
//! name alone, prepared against the seat's reach and never started here.

use std::ffi::OsStr;
use std::os::unix::fs::PermissionsExt;

use super::super::*;
use crate::broker::{Network, Reach, Refusal, Tree};

/// A canonicalised temporary root with a host HOME inside it.
struct Host {
    _dir: tempfile::TempDir,
    root: PathBuf,
    home: PathBuf,
}

impl Host {
    fn new() -> Host {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let home = root.join("home");
        std::fs::create_dir_all(&home).unwrap();
        Host {
            _dir: dir,
            root,
            home,
        }
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    /// A file at `relative` with `mode`.
    fn file(&self, relative: &str, mode: u32) -> PathBuf {
        let path = self.path(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "#!/bin/sh\n").unwrap();
        let mode = std::os::unix::fs::PermissionsExt::from_mode(mode);
        std::fs::set_permissions(&path, mode).unwrap();
        path
    }

    /// An executable at `relative`.
    fn plant(&self, relative: &str) -> PathBuf {
        self.file(relative, 0o755)
    }

    /// A symlink at `relative` to `target`.
    fn link(&self, relative: &str, target: &Path) -> PathBuf {
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

/// An installed executable inside the system set that is no system entry:
/// `/usr/lib/<package>/…/<file>` outside any `bin`, a package the system
/// set already holds (e.g. `/usr/lib/apt/methods/http`).
fn installed() -> (PathBuf, PathBuf) {
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
                & parent.starts_with("/usr/lib/")
                & !["bin", "sbin"].map(OsStr::new).contains(&named)
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
fn server_system() -> Vec<&'static str> {
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

/// The server box's whole argv for `etc`, `bootstrap` and the varying
/// pieces: the network flag, the resolver bind, the tree bind and the
/// command. Each system source linked outside the set is bound at its
/// canonical path too, as this host resolves it.
fn server_argv(etc: &Path, bootstrap: &Path, pieces: [&str; 4]) -> Vec<String> {
    let [net, dns, tree, run] = pieces;
    let mut argv: Vec<String> = format!(
        "bwrap --die-with-parent --unshare-pid --unshare-ipc --unshare-uts \
         --unshare-cgroup-try --cap-drop ALL {net} --clearenv --setenv {HANDS_BOX_ENV} 1 \
         --proc /proc --dev /dev --dir /runtime --dir /etc --dir /home --dir /root --dir /run \
         --dir /usr"
    )
    .split_whitespace()
    .map(String::from)
    .collect();
    let system = server_system();
    for host in &system {
        argv.extend(["--ro-bind-try", host, host].map(String::from));
    }
    let canonical = |source: &Path| std::fs::canonicalize(source).ok();
    for (host, alias) in namespace::aliases_with(&system, canonical) {
        let alias = alias.display().to_string();
        argv.extend(["--ro-bind-try".to_string(), host.to_string(), alias]);
    }
    let (e, b) = (etc.display(), bootstrap.display());
    argv.extend(
        format!(
            "--tmpfs {SANDBOX_HOME} --tmpfs /tmp {tree} --ro-bind {b} {b} {dns} \
             --ro-bind {e}/passwd /etc/passwd --ro-bind {e}/group /etc/group \
             --ro-bind {e}/hosts /etc/hosts --ro-bind {e}/nsswitch.conf /etc/nsswitch.conf \
             --setenv PATH /usr/local/bin:/usr/bin:/bin --setenv HOME {SANDBOX_HOME} \
             --setenv TMPDIR /tmp --setenv USER runner --setenv LOGNAME runner \
             --setenv LANG C.UTF-8 --setenv LC_ALL C.UTF-8 --chdir {SANDBOX_HOME} -- {run}"
        )
        .split_whitespace()
        .map(String::from),
    );
    argv
}

/// Where the box's generated identity lies: the source its passwd bind
/// names.
fn identity_dir(argv: &[String]) -> PathBuf {
    let bind = argv
        .windows(3)
        .find(|bind| bind[2] == "/etc/passwd")
        .unwrap();
    Path::new(&bind[1]).parent().unwrap().to_path_buf()
}

/// The box `program` is prepared in under `reach` and `network`, the test
/// binary its bootstrap, `arguments` its own.
fn prepared(
    program: &ServerProgram,
    reach: &Reach,
    network: &Network,
    arguments: &[String],
) -> Result<ServerBox, Refusal> {
    let bootstrap = std::env::current_exe().unwrap();
    let profile = ServerProfile {
        reach,
        network,
        bootstrap: &bootstrap,
        arguments,
    };
    ServerBox::prepare(program, &profile)
}

fn reach(writable: &[PathBuf], readable: &[PathBuf]) -> Reach {
    Reach {
        writable: writable.to_vec(),
        readable: readable.to_vec(),
    }
}

#[test]
fn the_server_box_holds_the_projected_system_set_and_its_own_private_paths() {
    let host = Host::new();
    let bootstrap = std::env::current_exe().unwrap();
    let seat = reach(&[host.path("work")], &[host.path("cache")]);
    // A dedicated package, isolated: its root bound, files-only names.
    let entry = host.plant("opt/docs/bin/docs-mcp");
    let docs = ServerProgram::resolve(entry.to_str().unwrap(), &host.home).unwrap();
    let stdio = ["--stdio".to_string()];
    let server = prepared(&docs, &seat, &Network::Isolated, &stdio).unwrap();
    let etc = identity_dir(server.argv());
    let tree = format!("--ro-bind {r} {r}", r = host.path("opt/docs").display());
    let run = format!("{} --stdio", entry.display());
    let isolated = ["--unshare-net", "", &tree, &run];
    let isolated_argv = server.argv().to_vec();
    assert_eq!(isolated_argv, server_argv(&etc, &bootstrap, isolated));
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
    let dns = "--ro-bind-try /etc/resolv.conf /etc/resolv.conf";
    let arguments = ["-c".to_string(), "true".to_string()];
    for (program, run) in [(&sh, found("sh")), (&installed, executable)] {
        let server = prepared(program, &seat, &Network::Shared, &arguments).unwrap();
        let etc = identity_dir(server.argv());
        let run = format!("{} -c true", run.display());
        let shared = ["", dns, "", &run];
        assert_eq!(server.argv(), server_argv(&etc, &bootstrap, shared));
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
    let host = Host::new();
    let sh = ServerProgram::resolve("sh", &host.home).unwrap();
    let server = prepared(&sh, &reach(&[], &[]), &Network::Isolated, &[]).unwrap();
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
        prepared(program, &seat, &Network::Shared, &[]).map(|_| ())
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
        (vec![host.path("work")], vec![host.path("cache")], Ok(())),
    ] {
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
    // the skeleton.
    let mut refused: Vec<PathBuf> = ["/usr", "/etc", "/etc/ssl", "/tmp", "/runtime", "/run"]
        .into_iter()
        .chain(["/proc", "/dev"])
        .map(PathBuf::from)
        .collect();
    // A root inside the source the projection narrowed away.
    refused.extend(std::fs::canonicalize("/etc/ssl").map(|ssl| ssl.join("private")));
    let closed = |namespace: Namespace| namespace.enter(Path::new("/"), &[]);
    let unbound = closed(Namespace::server(false));
    for root in refused {
        let mut namespace = Namespace::server(false);
        let answer = namespace.package(&Tree::Package { root: root.clone() });
        assert_eq!((&root, answer), (&root, Err(Refusal::ProgramTree)));
        assert_eq!(closed(namespace), unbound);
    }
    // A dedicated root, one on top of the private `/tmp`, and one the
    // system set holds, which gains no mount.
    for (root, bound) in [
        ("/opt/docs", true),
        ("/tmp/docs", true),
        ("/usr/lib/docs", false),
    ] {
        let mut namespace = Namespace::server(true);
        let tree = Tree::Package { root: root.into() };
        assert_eq!((root, namespace.package(&tree)), (root, Ok(())));
        let mut expected = closed(Namespace::server(true));
        let at = expected.len() - 3;
        let bind = ["--ro-bind", root, root].map(String::from);
        expected.splice(at..at, bind.into_iter().filter(|_| bound));
        assert_eq!((root, closed(namespace)), (root, expected));
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
        "[ \"$(pwd)\" = {SANDBOX_HOME} ] || exit 10; \
         [ -z \"$(ls -A {SANDBOX_HOME})\" ] && [ -z \"$(ls -A /tmp)\" ] || exit 11; \
         touch {SANDBOX_HOME}/a /tmp/b && [ ! -e /tmp/a ] || exit 12; \
         [ ! -e '{w}' ] && [ ! -e '{home}' ] || exit 13; \
         [ ! -e {p} ] || exit 14; {certs} \
         [ ! -e /etc/resolv.conf ] || exit 15; \
         [ \"$(cat /etc/nsswitch.conf)\" = 'hosts: files' ] || exit 16; \
         [ \"$PATH:$TMPDIR:$USER:${HANDS_BOX_ENV}\" = /usr/local/bin:/usr/bin:/bin:/tmp:runner:1 ] \
         || exit 18; {aliased} true",
        w = work.display(),
        p = private.display(),
    );
    let arguments = ["-c".to_string(), script];
    let sh = ServerProgram::resolve("sh", &host.home).unwrap();
    let bootstrap = shell();
    let seat = reach(&[work], &[]);
    let profile = ServerProfile {
        reach: &seat,
        network: &Network::Isolated,
        bootstrap: &bootstrap,
        arguments: &arguments,
    };
    let server = ServerBox::prepare(&sh, &profile).unwrap();
    // The host's private TLS directory, unreadable here or absent, was
    // neither read nor mounted.
    let unreadable = private.exists() & std::fs::read_dir(private).is_err();
    eprintln!("/etc/ssl/private unreadable on this host: {unreadable}");
    let status = Command::new(require_bwrap().unwrap())
        .args(&server.argv()[1..])
        .current_dir("/")
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(0));
}
