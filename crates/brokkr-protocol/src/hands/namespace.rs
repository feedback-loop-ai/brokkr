//! The namespace a box is built in, and the bubblewrap argv that says so
//! (decision 0065 slice two, U6c3): the isolation flags, the empty root's
//! skeleton, the host toolchain read-only, the generated identity, and the
//! environment and command that close the argv. What a box mounts beyond
//! that is its profile's: the workspace's workdir, git and declared binds
//! stay with `box_argv`.

use std::path::Path;

use super::{ids, HANDS_BOX_ENV, HOST_TOOLCHAIN_BINDS, SANDBOX_HOME};

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

/// One box's bubblewrap argv, built in mount order: a later mount shadows
/// an earlier one, so the order the calls are made in is the boundary.
pub(super) struct Namespace {
    argv: Vec<String>,
}

impl Namespace {
    /// The isolation every box opens with, the empty root's skeleton, and
    /// the host toolchain read-only where it exists (`-try`: an absent
    /// source is skipped, never an error — /lib64 is a Debian fact, not a
    /// law). The network is unshared unless `network` grants it.
    pub(super) fn open(network: bool) -> Namespace {
        let mut argv: Vec<String> = [
            "bwrap",
            "--die-with-parent",
            "--new-session",
            "--unshare-pid",
            "--unshare-ipc",
            "--unshare-uts",
            "--unshare-cgroup-try",
            "--cap-drop",
            "ALL",
        ]
        .map(String::from)
        .to_vec();
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
        let mut namespace = Namespace { argv };
        for host in HOST_TOOLCHAIN_BINDS {
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
        std::fs::create_dir_all(etc)?;
        let (uid, gid) = ids();
        let passwd = format!("runner:x:{uid}:{gid}:brokkr hands:{SANDBOX_HOME}:/bin/sh\n");
        let group = format!("runner:x:{gid}:\n");
        let hosts = "127.0.0.1 localhost\n::1 localhost ip6-localhost ip6-loopback\n";
        for (name, text) in [
            ("passwd", passwd.as_str()),
            ("group", group.as_str()),
            ("hosts", hosts),
            ("nsswitch.conf", "hosts: files\n"),
        ] {
            std::fs::write(etc.join(name), text)?;
        }
        for (name, target) in [
            ("passwd", "/etc/passwd"),
            ("group", "/etc/group"),
            ("hosts", "/etc/hosts"),
            ("nsswitch.conf", "/etc/nsswitch.conf"),
        ] {
            self.mount(Mount::RoBind, &etc.join(name), target);
        }
        Ok(())
    }

    /// Mount `host` at `target` inside the box.
    pub(super) fn mount(&mut self, mount: Mount, host: &Path, target: &str) {
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
