//! Node resolution against planted layouts: a temporary HOME and PATH,
//! never the host's own volta, nvm or system node.

use std::ffi::OsString;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};

use super::{reach, Found, Install, Layout, Reach, Unbound};
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

    /// An empty executable file at `relative` under the root, as an
    /// installer leaves a binary.
    fn file(&self, relative: &str) -> PathBuf {
        self.file_of(relative, 0o755)
    }

    /// An empty file at `relative` under the root that no one may run.
    fn unexecutable(&self, relative: &str) -> PathBuf {
        self.file_of(relative, 0o644)
    }

    fn file_of(&self, relative: &str, mode: u32) -> PathBuf {
        let path = self.root.path().canonicalize().unwrap().join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
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
        Some(Reach::Unbindable("pnpm", home.clone(), Unbound::TooBroad))
    );
    let local = machine.reach("pnpm", &["home/.local/bin"]).unwrap();
    assert_eq!(
        local,
        Reach::Unbindable("pnpm", home.join(".local"), Unbound::TooBroad)
    );
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
    assert_eq!(
        found.add(slash, Path::new("")),
        Err((PathBuf::from("/"), Unbound::TooBroad))
    );
}

/// The refusal `refusal` gives for `binary` under `root`, which holds
/// `entry` that `layout`, named in words, does not, and the verifier
/// lines that fail closed with it.
fn foreign(binary: &str, root: &Path, entry: &str, layout: &str) -> (String, String) {
    let said = format!(
        "{binary} is installed under {}, which holds '{entry}', no part of {layout}, \
         and which `brokkr init` will not bind into verify's hands; install it under \
         a directory of its own, or fill in .brokkr/scripts/verify-seat.sh",
        root.display()
    );
    let quoted = said.replace('\'', "'\\''");
    let script = format!("printf '%s' '{quoted}' > \"$notes\"\nwrite_result fail; exit 0\n");
    (said, script)
}

/// The chief's reproduction (b) of #717's security hold: npm and node
/// planted in `~/.aws/bin`, beside `~/.aws/credentials`. The walk from
/// the `bin` reaches `~/.aws`, which a node prefix does not describe, so
/// it is refused naming the credentials, where the story bound `~/.aws`.
#[test]
fn an_install_beside_credentials_is_refused_naming_them() {
    let machine = Machine::new();
    machine.file("home/.aws/bin/npm");
    machine.file("home/.aws/bin/node");
    machine.file("home/.aws/credentials");
    let refused = machine.reach("npm", &["home/.aws/bin"]).unwrap();
    let aws = machine.home().join(".aws");
    let credentials = Unbound::Foreign(Layout::NodePrefix, OsString::from("credentials"));
    assert_eq!(refused, Reach::Unbindable("npm", aws.clone(), credentials));
    let (said, script) = foreign("npm", &aws, "credentials", "a node prefix");
    assert_eq!(refused.refusal(), Some(said));
    assert_eq!(refused.script(), script);
    assert_eq!(refused.binds(), Vec::<serde_json::Value>::new());
}

/// The chief's reproduction (a): a standalone pnpm in its home,
/// `~/.local/share/pnpm`, beside `~/.local/share/keyrings`. pnpm's home
/// is the directory `PATH` names, so it, holding pnpm alone, is what is
/// bound, never `~/.local/share` and its keyring; with one entry pnpm's
/// home does not name, the home is refused too.
#[test]
fn a_standalone_pnpm_binds_its_home_and_never_its_parent() {
    let machine = Machine::new();
    machine.file("home/.local/share/pnpm/pnpm");
    machine.file("home/.local/share/keyrings/user.keystore");
    machine.file("opt/node/bin/node");
    let dirs = ["home/.local/share/pnpm", "opt/node/bin"];
    let pnpm = machine.home().join(".local/share/pnpm");
    assert_eq!(
        machine.reach("pnpm", &dirs),
        Some(Reach::Found {
            binds: vec![
                "~/.local/share/pnpm".to_string(),
                machine.dir("opt/node").display().to_string()
            ],
            path: vec![pnpm.clone(), machine.dir("opt/node/bin")],
            volta: None,
        })
    );
    machine.file("home/.local/share/pnpm/.npmrc");
    let refused = machine.reach("pnpm", &dirs).unwrap();
    let npmrc = Unbound::Foreign(Layout::PnpmHome, OsString::from(".npmrc"));
    assert_eq!(refused, Reach::Unbindable("pnpm", pnpm.clone(), npmrc));
    let (said, script) = foreign("pnpm", &pnpm, ".npmrc", "pnpm's home");
    assert_eq!((refused.refusal(), refused.script()), (Some(said), script));
}

/// A plain-file runner outside HOME, in an `/opt`-like `bin` whose parent
/// holds an `etc` beside it, binds no directory: the parent is refused
/// naming the `etc`, as a root under `/opt` holding other installs is.
#[test]
fn a_plain_runner_outside_home_binds_no_directory_holding_others() {
    let machine = Machine::new();
    machine.file("opt/bin/npm");
    machine.file("opt/bin/node");
    machine.file("opt/etc/npmrc");
    let opt = machine.dir("opt");
    let etc = Unbound::Foreign(Layout::NodePrefix, OsString::from("etc"));
    assert_eq!(
        machine.reach("npm", &["opt/bin"]),
        Some(Reach::Unbindable("npm", opt, etc))
    );
}

/// A root whose entries cannot be listed is refused, not bound: `init`
/// cannot tell what it holds.
#[test]
fn a_root_that_cannot_be_listed_is_refused() {
    let machine = Machine::new();
    machine.file("opt/node/bin/npm");
    machine.file("opt/node/bin/node");
    let node = machine.dir("opt/node");
    let listable = |mode| std::fs::set_permissions(&node, std::fs::Permissions::from_mode(mode));
    listable(0o311).unwrap();
    let refused = machine.reach("npm", &["opt/node/bin"]);
    listable(0o755).unwrap();
    let refused = refused.unwrap();
    assert_eq!(
        refused,
        Reach::Unbindable("npm", node.clone(), Unbound::Unreadable)
    );
    assert_eq!(
        refused.refusal().unwrap(),
        format!(
            "npm is installed under {}, whose entries could not be read, and which \
             `brokkr init` will not bind into verify's hands; install it under a \
             directory of its own, or fill in .brokkr/scripts/verify-seat.sh",
            node.display()
        )
    );
}

/// One recognised layout as its installer leaves it: the runner, the
/// `PATH` directory, the root bound, the binaries and links planted, and
/// every top-level entry the layout names.
struct Planted {
    runner: &'static str,
    bin: &'static str,
    root: &'static str,
    layout: (Layout, &'static str),
    files: &'static [&'static str],
    links: &'static [(&'static str, &'static str)],
    entries: &'static [&'static str],
}

const PREFIX: &[&str] = &[
    "CHANGELOG.md",
    "LICENSE",
    "README.md",
    "bin",
    "include",
    "lib",
    "share",
];
const NVM_LINK: (&str, &str) = ("bin/npm", "../lib/node_modules/npm/bin/npm-cli.js");
const NODE_FILES: &[&str] = &["bin/node", "lib/node_modules/npm/bin/npm-cli.js"];

const PLANTED: &[Planted] = &[
    Planted {
        runner: "npm",
        bin: "home/.volta/bin",
        root: "home/.volta",
        layout: (Layout::Volta, "volta's root"),
        files: &["bin/volta-shim", "tools/image/node/20.11.0/bin/node"],
        links: &[("bin/npm", "volta-shim"), ("bin/node", "volta-shim")],
        entries: &[
            "bin",
            "cache",
            "hooks.json",
            "layout.v1",
            "layout.v2",
            "layout.v3",
            "layout.v4",
            "log",
            "tmp",
            "tools",
        ],
    },
    Planted {
        runner: "npm",
        bin: "home/.nvm/versions/node/v20.11.0/bin",
        root: "home/.nvm/versions/node/v20.11.0",
        layout: (Layout::NodePrefix, "a node prefix"),
        files: NODE_FILES,
        links: &[NVM_LINK],
        entries: PREFIX,
    },
    Planted {
        runner: "npm",
        bin: "home/.local/share/fnm/node-versions/v20.11.0/installation/bin",
        root: "home/.local/share/fnm/node-versions/v20.11.0/installation",
        layout: (Layout::NodePrefix, "a node prefix"),
        files: NODE_FILES,
        links: &[NVM_LINK],
        entries: PREFIX,
    },
    Planted {
        runner: "bun",
        bin: "home/.bun/bin",
        root: "home/.bun",
        layout: (Layout::Bun, "bun's root"),
        files: &["bin/bun", "install/global/package.json"],
        links: &[("bin/bunx", "bun")],
        entries: &["_bun", "bin", "install"],
    },
    Planted {
        runner: "pnpm",
        bin: "home/.local/share/pnpm",
        root: "home/.local/share/pnpm",
        layout: (Layout::PnpmHome, "pnpm's home"),
        files: &["pnpm", "nodejs/20.11.0/bin/node"],
        links: &[("node", "nodejs/20.11.0/bin/node")],
        entries: &[
            "global", "node", "nodejs", "npm", "npx", "pnpm", "pnpx", "store",
        ],
    },
];

/// Each recognised layout, planted with exactly the entries it names,
/// binds its root whole; one `credentials` beside them and the root is
/// refused, naming it.
#[test]
fn each_layout_binds_whole_until_it_holds_one_entry_more() {
    for planted in PLANTED {
        let machine = Machine::new();
        let at = |relative: &str| format!("{}/{relative}", planted.root);
        for file in planted.files {
            machine.file(&at(file));
        }
        for (link, target) in planted.links {
            machine.link(&at(link), target);
        }
        for entry in planted.entries {
            if !machine.dir(&at(entry)).exists() {
                machine.unexecutable(&at(entry));
            }
        }
        let root = machine.dir(planted.root);
        let held = std::fs::read_dir(&root).unwrap().count();
        assert_eq!(held, planted.entries.len(), "{}", planted.root);
        let found = machine.reach(planted.runner, &[planted.bin]).unwrap();
        let Reach::Found { binds, .. } = &found else {
            panic!("{}: {found:?}", planted.root);
        };
        let spelled = planted.root.replacen("home/", "~/", 1);
        assert_eq!(binds, &vec![spelled], "{}", planted.root);
        machine.unexecutable(&at("credentials"));
        let refused = machine.reach(planted.runner, &[planted.bin]).unwrap();
        let (layout, words) = planted.layout;
        let credentials = Unbound::Foreign(layout, OsString::from("credentials"));
        assert_eq!(
            refused,
            Reach::Unbindable(planted.runner, root.clone(), credentials)
        );
        let said = foreign(planted.runner, &root, "credentials", words).0;
        assert_eq!(refused.refusal(), Some(said));
    }
}

/// A non-executable `npm` earlier on `PATH` is passed over for the
/// executable one after it, as the shell's lookup passes it; with only
/// the non-executable one, npm is not found.
#[test]
fn a_runner_that_cannot_run_is_passed_over_on_path() {
    let machine = Machine::new();
    machine.unexecutable("home/bin/npm");
    machine.file("opt/node/bin/npm");
    machine.file("opt/node/bin/node");
    assert_eq!(
        machine.reach("npm", &["home/bin", "opt/node/bin"]),
        Some(Reach::Found {
            binds: vec![machine.dir("opt/node").display().to_string()],
            path: vec![machine.dir("opt/node/bin")],
            volta: None,
        })
    );
    assert_eq!(
        machine.reach("npm", &["home/bin"]),
        Some(Reach::Missing("npm"))
    );
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
