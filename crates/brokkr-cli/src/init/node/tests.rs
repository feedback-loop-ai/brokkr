//! Node resolution against planted layouts: a temporary HOME and PATH,
//! never the host's own volta, nvm or system node.

use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use super::{reach, Found, Install, Reach};
use crate::init::{tools_for, Tool};

/// A planted machine: `home` under a temporary root, and files made
/// under either.
struct Machine {
    root: tempfile::TempDir,
}

impl Machine {
    fn new() -> Machine {
        let machine = Machine {
            root: tempfile::tempdir().unwrap(),
        };
        std::fs::create_dir(machine.home()).unwrap();
        machine
    }

    /// The planted HOME, canonical, as `reach` reads it.
    fn home(&self) -> PathBuf {
        self.root.path().canonicalize().unwrap().join("home")
    }

    /// An empty file at `relative` under the root.
    fn file(&self, relative: &str) -> PathBuf {
        let path = self.root.path().canonicalize().unwrap().join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "").unwrap();
        path
    }

    /// A link at `relative` under the root to `target`.
    fn link(&self, relative: &str, target: &str) {
        symlink(target, self.root.path().join(relative)).unwrap();
    }

    fn dir(&self, relative: &str) -> PathBuf {
        self.root.path().canonicalize().unwrap().join(relative)
    }

    /// What `reach` makes of `runner` on a PATH of `dirs` under the root.
    fn reach(&self, runner: &str, dirs: &[&str]) -> Option<Reach> {
        let path = std::env::join_paths(dirs.iter().map(|dir| self.dir(dir))).unwrap();
        reach(&tools_for(runner), &path, &self.home())
    }
}

/// A volta shim: `bin/npm` and `bin/node` link to `bin/volta-shim`, which
/// execs from `tools/image`. The volta root is bound, its `bin` heads
/// PATH, and `VOLTA_HOME` names the root the box's own HOME would hide.
#[test]
fn a_volta_shim_binds_the_volta_root() {
    let machine = Machine::new();
    machine.file("home/.volta/bin/volta-shim");
    machine.link("home/.volta/bin/npm", "volta-shim");
    machine.link("home/.volta/bin/node", "volta-shim");
    machine.file("home/.volta/tools/image/node/20.11.0/bin/node");
    machine.file("home/.volta/tools/image/node/20.11.0/bin/npm");
    let found = machine.reach("npm", &["home/.volta/bin"]).unwrap();
    assert_eq!(
        found,
        Reach::Found {
            binds: vec!["~/.volta".to_string()],
            path: vec![machine.home().join(".volta/bin")],
            volta: Some(machine.home().join(".volta")),
        }
    );
    assert_eq!(
        found.binds(),
        [serde_json::json!({"path": "~/.volta", "mode": "ro"})]
    );
    let home = machine.home().display().to_string();
    assert_eq!(
        found.script(),
        format!("export PATH='{home}/.volta/bin':\"$PATH\"\nexport VOLTA_HOME='{home}/.volta'\n")
    );
    assert_eq!(found.refusal(), None);
}

/// nvm: `bin/npm` links into the version's own `lib`, beside `bin/node`;
/// the version directory holds both and is the one bound.
#[test]
fn an_nvm_install_binds_its_version_directory() {
    let machine = Machine::new();
    let version = "home/.nvm/versions/node/v20.11.0";
    machine.file(&format!("{version}/bin/node"));
    machine.file(&format!("{version}/lib/node_modules/npm/bin/npm-cli.js"));
    machine.link(
        &format!("{version}/bin/npm"),
        "../lib/node_modules/npm/bin/npm-cli.js",
    );
    let found = machine.reach("npm", &[&format!("{version}/bin")]);
    assert_eq!(
        found,
        Some(Reach::Found {
            binds: vec!["~/.nvm/versions/node/v20.11.0".to_string()],
            path: vec![machine.dir(&format!("{version}/bin"))],
            volta: None,
        })
    );
}

/// A runner under a directory outside HOME binds its own install root by
/// its absolute path, and nothing of HOME; bun runs on no `node`.
#[test]
fn a_runner_outside_home_binds_no_home_directory() {
    let machine = Machine::new();
    machine.file("opt/node/bin/npm");
    machine.file("opt/node/bin/node");
    machine.file("opt/bun/bin/bun");
    let opt = |install: &str| machine.dir(install).display().to_string();
    assert_eq!(
        machine.reach("npm", &["opt/node/bin"]),
        Some(Reach::Found {
            binds: vec![opt("opt/node")],
            path: vec![machine.dir("opt/node/bin")],
            volta: None,
        })
    );
    // With no HOME at all, nothing is spelled under one.
    let path = machine.dir("opt/bun/bin").into_os_string();
    assert_eq!(
        reach(&tools_for("bun"), &path, Path::new("")),
        Some(Reach::Found {
            binds: vec![opt("opt/bun")],
            path: vec![machine.dir("opt/bun/bin")],
            volta: None,
        })
    );
}

/// No runner on PATH, or no `node` for one that runs on it: init's line
/// and the fail-closed verify text name the binary. A PATH entry that is
/// missing, or holds a directory of the runner's name, finds nothing.
#[test]
fn a_runner_not_on_path_fails_verify_closed() {
    let machine = Machine::new();
    std::fs::create_dir_all(machine.dir("dir/npm")).unwrap();
    let missing = machine.reach("npm", &["absent", "dir"]).unwrap();
    assert_eq!(missing, Reach::Missing("npm"));
    let said = "npm was not found on PATH when `brokkr init` ran; install it, \
                or fill in .brokkr/scripts/verify-seat.sh";
    assert_eq!(missing.refusal().as_deref(), Some(said));
    assert_eq!(
        missing.script(),
        format!("printf '%s' '{said}' > \"$notes\"\nwrite_result fail; exit 0\n")
    );
    assert_eq!(missing.binds(), Vec::<serde_json::Value>::new());
    machine.file("opt/yarn/bin/yarn");
    assert_eq!(
        machine.reach("yarn", &["opt/yarn/bin"]),
        Some(Reach::Missing("node"))
    );
}

/// An install rooted at HOME, or at `~/.local` whole, is refused rather
/// than bound, and verify fails closed naming where it stands.
#[test]
fn a_root_too_broad_to_bind_is_refused() {
    let machine = Machine::new();
    machine.file("home/bin/pnpm");
    machine.file("home/.local/bin/pnpm");
    let home = machine.home();
    assert_eq!(
        machine.reach("pnpm", &["home/bin"]),
        Some(Reach::Unbindable("pnpm", home.clone()))
    );
    let local = machine.reach("pnpm", &["home/.local/bin"]).unwrap();
    assert_eq!(local, Reach::Unbindable("pnpm", home.join(".local")));
    assert_eq!(
        local.refusal().unwrap(),
        format!(
            "pnpm is installed under {}, which `brokkr init` will not bind into \
             verify's hands; install it under a directory of its own, or fill in \
             .brokkr/scripts/verify-seat.sh",
            home.join(".local").display()
        )
    );
    // `/` itself, where no HOME was known to rule it out.
    let mut found = Found::default();
    let slash = Install {
        dir: PathBuf::from("/"),
        real: PathBuf::from("/npm"),
    };
    assert_eq!(found.add(slash, Path::new("")), Err(PathBuf::from("/")));
}

/// A binary whose directory and file stand under a toolchain root every
/// box reads adds no bind, only its directory to PATH.
#[test]
fn a_runner_every_box_reads_adds_no_bind() {
    let mut found = Found::default();
    let system = Install {
        dir: PathBuf::from("/usr/bin"),
        real: PathBuf::from("/usr/share/nodejs/npm/bin/npm-cli.js"),
    };
    assert_eq!(found.add(system, Path::new("/home/someone")), Ok(()));
    assert_eq!(
        (found.binds, found.path, found.volta),
        (Vec::<String>::new(), vec![PathBuf::from("/usr/bin")], None)
    );
}

/// A stack with no node-family runner resolves nothing.
#[test]
fn a_stack_with_no_node_runner_resolves_nothing() {
    let machine = Machine::new();
    let runners: Vec<Tool> = ["cargo", "make"].into_iter().flat_map(tools_for).collect();
    let path = machine.dir("bin").into_os_string();
    assert_eq!(reach(&runners, &path, &machine.home()), None);
}
