//! Node resolution against planted layouts: a temporary HOME and PATH,
//! never the host's own volta, nvm or system node, and the tools' homes
//! named as on Linux, by no variable unless a test sets one.

use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};

use super::{reach, Flaw, Found, Homes, Install, Layout, Reach, Unbound};
use crate::init::{tools_for, Tool};

/// The homes named where no HOME and no variable is.
fn nowhere() -> Homes {
    Homes::named(Path::new(""), "linux", |_| None)
}

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
        let path = self.root.path().join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        symlink(target, path).unwrap();
    }

    /// A node prefix at `prefix` as a tarball leaves it: `bin/node`, and
    /// `bin/npm` linking into `lib/node_modules`.
    fn linked_prefix(&self, prefix: &str) {
        self.file(&format!("{prefix}/bin/node"));
        self.file(&format!("{prefix}/lib/node_modules/npm/bin/npm-cli.js"));
        self.link(&format!("{prefix}/bin/npm"), NVM_LINK.1);
    }

    /// `relative` under the root, as `reach` spells a bind outside HOME.
    fn spelled(&self, relative: &str) -> String {
        self.dir(relative).display().to_string()
    }

    fn dir(&self, relative: &str) -> PathBuf {
        self.root.path().canonicalize().unwrap().join(relative)
    }

    /// The homes named on Linux under the planted HOME, with `vars` the
    /// only variables set.
    fn homes(&self, vars: &[(&str, OsString)]) -> Homes {
        let var = |name: &str| vars.iter().find(|(set, _)| *set == name);
        Homes::named(&self.home(), "linux", |name| {
            var(name).map(|(_, value)| value.clone())
        })
    }

    /// What `reach` makes of `runner` on a PATH of `dirs` under the root.
    fn reach(&self, runner: &str, dirs: &[&str]) -> Option<Reach> {
        self.reach_named(runner, dirs, &[])
    }

    /// [`Machine::reach`] with `vars` naming the tools' homes.
    fn reach_named(&self, runner: &str, dirs: &[&str], vars: &[(&str, OsString)]) -> Option<Reach> {
        let path = std::env::join_paths(dirs.iter().map(|dir| self.dir(dir))).unwrap();
        reach(&tools_for(runner), &path, &self.homes(vars))
    }

    /// `relative` under the root as `reach` spells a bind: `~/`-spelled
    /// under HOME.
    fn spell(&self, relative: &str) -> String {
        match relative.strip_prefix("home/") {
            Some(rest) => format!("~/{rest}"),
            None => self.spelled(relative),
        }
    }

    /// `relative` under the root as a variable's value.
    fn var(&self, relative: &str) -> OsString {
        self.dir(relative).into_os_string()
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
/// the version's `bin` and `lib/node_modules` are bound, and `bin/node`,
/// a file under the `bin` bind, is not bound a second time.
#[test]
fn an_nvm_install_binds_its_bin_and_node_modules() {
    let machine = Machine::new();
    let version = "home/.nvm/versions/node/v20.11.0";
    machine.linked_prefix(version);
    let found = machine.reach("npm", &[&format!("{version}/bin")]);
    assert_eq!(
        found,
        Some(Reach::Found {
            binds: vec![
                "~/.nvm/versions/node/v20.11.0/bin".to_string(),
                "~/.nvm/versions/node/v20.11.0/lib/node_modules".to_string(),
            ],
            path: vec![machine.dir(&format!("{version}/bin"))],
            volta: None,
        })
    );
}

/// A runner under a directory outside HOME binds its own files by their
/// absolute paths, and nothing of HOME; bun runs on no `node`.
#[test]
fn a_runner_outside_home_binds_no_home_directory() {
    let machine = Machine::new();
    machine.file("opt/node/bin/npm");
    machine.file("opt/node/bin/node");
    machine.file("opt/bun/bin/bun");
    assert_eq!(
        machine.reach("npm", &["opt/node/bin"]),
        Some(Reach::Found {
            binds: vec![
                machine.spelled("opt/node/bin/npm"),
                machine.spelled("opt/node/bin/node")
            ],
            path: vec![machine.dir("opt/node/bin")],
            volta: None,
        })
    );
    // With no HOME at all, nothing is spelled under one.
    let path = machine.dir("opt/bun/bin").into_os_string();
    assert_eq!(
        reach(&tools_for("bun"), &path, &nowhere()),
        Some(Reach::Found {
            binds: vec![machine.spelled("opt/bun/bin/bun")],
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
    let cjs = "../lib/node_modules/pnpm/bin/pnpm.cjs";
    machine.file("home/lib/node_modules/pnpm/bin/pnpm.cjs");
    machine.link("home/bin/pnpm", cjs);
    machine.file("home/.local/lib/node_modules/pnpm/bin/pnpm.cjs");
    machine.link("home/.local/bin/pnpm", cjs);
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
    // `/` itself, a prefix above `/bin`, where no HOME was known to rule
    // it out.
    let mut found = Found::default();
    let slash = Install {
        dir: PathBuf::from("/bin"),
        entry: PathBuf::from("/bin/npm"),
        real: PathBuf::from("/npm-cli/npm"),
    };
    assert_eq!(
        found.add(slash, &nowhere()),
        Err((PathBuf::from("/"), Unbound::TooBroad))
    );
}

/// A home its tool's own variable names is still refused where it is
/// `$HOME` or `~/.local` whole: `PNPM_HOME=$HOME`, `VOLTA_HOME=~/.local`.
#[test]
fn a_home_too_broad_is_refused_even_where_its_variable_names_it() {
    let machine = Machine::new();
    machine.file("home/global/pnpm.cjs");
    machine.link("home/pnpm", "global/pnpm.cjs");
    machine.file("home/.local/bin/volta-shim");
    machine.link("home/.local/bin/npm", "volta-shim");
    let home = machine.home();
    let named = [("PNPM_HOME", home.clone().into_os_string())];
    assert_eq!(
        machine.reach_named("pnpm", &["home"], &named),
        Some(Reach::Unbindable("pnpm", home.clone(), Unbound::TooBroad))
    );
    let named = [("VOLTA_HOME", machine.var("home/.local"))];
    assert_eq!(
        machine.reach_named("npm", &["home/.local/bin"], &named),
        Some(Reach::Unbindable(
            "npm",
            home.join(".local"),
            Unbound::TooBroad
        ))
    );
}

/// The refusal `refusal` gives for `binary` under `root`, which holds
/// `entry` that `layout`, named in words, does not, and the verifier
/// lines that fail closed with it.
fn foreign(binary: &str, root: &Path, entry: &str, layout: &str) -> (String, String) {
    let which = format!("which holds '{entry}', no part of {layout}, and which");
    sentence(binary, root, &which)
}

/// The refusal for `binary` under `root`, its clause `which`, and the
/// verifier lines that fail closed with it.
fn sentence(binary: &str, root: &Path, which: &str) -> (String, String) {
    let said = format!(
        "{binary} is installed under {}, {which} `brokkr init` will not bind into \
         verify's hands; install it under a directory of its own, or fill in \
         .brokkr/scripts/verify-seat.sh",
        root.display()
    );
    let quoted = said.replace('\'', "'\\''");
    let script = format!("printf '%s' '{quoted}' > \"$notes\"\nwrite_result fail; exit 0\n");
    (said, script)
}

/// The chief's reproduction (b) of #717's security hold: npm and node
/// planted in `~/.aws/bin`, beside `~/.aws/credentials`. Where `bin/npm`
/// links into `~/.aws/lib/node_modules`, the walk reaches `~/.aws`, which
/// a node prefix does not describe, so it is refused naming the
/// credentials. Where both are plain files, each is bound alone: no
/// directory holding the credentials is bound, `~/.aws` or its `bin`.
#[test]
fn an_install_beside_credentials_is_refused_naming_them() {
    let machine = Machine::new();
    machine.linked_prefix("home/.aws");
    machine.file("home/.aws/credentials");
    let refused = machine.reach("npm", &["home/.aws/bin"]).unwrap();
    let aws = machine.home().join(".aws");
    let credentials = Unbound::Foreign(Layout::NodePrefix, OsString::from("credentials"));
    assert_eq!(refused, Reach::Unbindable("npm", aws.clone(), credentials));
    let (said, script) = foreign("npm", &aws, "credentials", "a node prefix");
    assert_eq!(refused.refusal(), Some(said));
    assert_eq!(refused.script(), script);
    assert_eq!(refused.binds(), Vec::<serde_json::Value>::new());
    std::fs::remove_file(aws.join("bin/npm")).unwrap();
    machine.file("home/.aws/bin/npm");
    let plain = machine.reach("npm", &["home/.aws/bin"]).unwrap();
    let Reach::Found { binds, .. } = plain else {
        panic!("the plain-file form is bound by its files: {plain:?}");
    };
    assert_eq!(binds, ["~/.aws/bin/npm", "~/.aws/bin/node"]);
}

/// The chief's reproduction (a): a standalone pnpm in its home,
/// `~/.local/share/pnpm`, beside `~/.local/share/keyrings`. The pnpm file
/// stands in the directory `PATH` names, so it alone is bound, never
/// `~/.local/share` and its keyring, nor the home and the `.npmrc` it
/// comes to hold.
#[test]
fn a_standalone_pnpm_binds_its_file_and_never_its_home() {
    let machine = Machine::new();
    machine.file("home/.local/share/pnpm/pnpm");
    machine.file("home/.local/share/keyrings/user.keystore");
    machine.file("opt/node/bin/node");
    let dirs = ["home/.local/share/pnpm", "opt/node/bin"];
    let pnpm = machine.home().join(".local/share/pnpm");
    let found = Reach::Found {
        binds: vec![
            "~/.local/share/pnpm/pnpm".to_string(),
            machine.spelled("opt/node/bin/node"),
        ],
        path: vec![pnpm, machine.dir("opt/node/bin")],
        volta: None,
    };
    assert_eq!(machine.reach("pnpm", &dirs).as_ref(), Some(&found));
    machine.file("home/.local/share/pnpm/.npmrc");
    assert_eq!(machine.reach("pnpm", &dirs), Some(found));
}

/// The chief's reproduction of the second hold: plain-file `npm` and
/// `node` in an `/opt`-like `bin` whose `share` holds a keystore. The
/// prefix is never walked to: the two files are bound, and no directory,
/// each spelled in `bundle.json` as a directory bind is, and the verifier
/// puts their `bin` at the head of `PATH`.
#[test]
fn a_plain_runner_in_a_shared_prefix_binds_its_files_alone() {
    let machine = Machine::new();
    machine.file("opt/bin/npm");
    machine.file("opt/bin/node");
    machine.file("opt/share/keyrings/user.keystore");
    let (npm, node) = (
        machine.spelled("opt/bin/npm"),
        machine.spelled("opt/bin/node"),
    );
    let found = machine.reach("npm", &["opt/bin"]).unwrap();
    assert_eq!(
        found,
        Reach::Found {
            binds: vec![npm.clone(), node.clone()],
            path: vec![machine.dir("opt/bin")],
            volta: None,
        }
    );
    assert_eq!(
        found.binds(),
        [
            serde_json::json!({"path": npm, "mode": "ro"}),
            serde_json::json!({"path": node, "mode": "ro"}),
        ]
    );
    let bin = machine.spelled("opt/bin");
    assert_eq!(found.script(), format!("export PATH='{bin}':\"$PATH\"\n"));
}

/// The link variant: `opt/bin/npm` links into `opt/lib/node_modules`, so
/// `opt` is a node prefix, and only its `bin` and `lib/node_modules` are
/// bound, never `opt` and the keystore its `share` holds.
#[test]
fn a_linked_npm_binds_its_prefix_bin_and_node_modules_alone() {
    let machine = Machine::new();
    machine.linked_prefix("opt");
    machine.file("opt/share/keyrings/user.keystore");
    assert_eq!(
        machine.reach("npm", &["opt/bin"]),
        Some(Reach::Found {
            binds: vec![
                machine.spelled("opt/bin"),
                machine.spelled("opt/lib/node_modules")
            ],
            path: vec![machine.dir("opt/bin")],
            volta: None,
        })
    );
}

/// A node prefix's `bin` holding anything but `node` and links into
/// `lib/node_modules` is refused, naming the entry under `bin`.
#[test]
fn a_prefix_bin_holding_another_entry_is_refused_naming_it() {
    let machine = Machine::new();
    machine.linked_prefix("opt");
    machine.file("opt/bin/credentials");
    let refused = machine.reach("npm", &["opt/bin"]).unwrap();
    let opt = machine.dir("opt");
    let credentials = Unbound::Foreign(Layout::NodePrefix, OsString::from("bin/credentials"));
    assert_eq!(refused, Reach::Unbindable("npm", opt.clone(), credentials));
    fails_closed(
        &refused,
        foreign("npm", &opt, "bin/credentials", "a node prefix"),
    );
}

/// `refused` says init's sentence, the verifier fails closed with it, and
/// it binds nothing, as [`sentence`] spells them.
fn fails_closed(refused: &Reach, (said, script): (String, String)) {
    assert_eq!((refused.refusal(), refused.script()), (Some(said), script));
    assert_eq!(refused.binds(), Vec::<serde_json::Value>::new());
}

/// The review's reproduction: `opt/bin/npm` reaches `opt/lib/node_modules`
/// through a link in another `PATH` directory, which the box, binding
/// `opt/bin` and `opt/lib/node_modules` alone, does not hold, so it is
/// refused naming `bin/npm`, absolute or relative, outside HOME or under
/// it; reached directly, the same prefix is bound.
#[test]
fn a_prefix_link_through_another_directory_is_refused_naming_it() {
    let machine = Machine::new();
    for prefix in ["opt", "home/opt"] {
        machine.linked_prefix(prefix);
        let cli = machine.dir(&format!("{prefix}/lib/node_modules/npm/bin/npm-cli.js"));
        let second = machine.dir(&format!("{prefix}-second/bin/npm"));
        machine.link(&format!("{prefix}-second/bin/npm"), cli.to_str().unwrap());
        let leaf = prefix.rsplit('/').next().unwrap();
        let relative = format!("../{leaf}/lib/node_modules/npm/bin/npm-cli.js");
        machine.link(&format!("{prefix}-links/npm"), &relative);
        let dirs = [format!("{prefix}/bin"), format!("{prefix}-second/bin")];
        let dirs: Vec<&str> = dirs.iter().map(String::as_str).collect();
        assert_eq!(
            machine.reach("npm", &dirs),
            Some(Reach::Found {
                binds: vec![
                    machine.spell(&format!("{prefix}/bin")),
                    machine.spell(&format!("{prefix}/lib/node_modules")),
                ],
                path: dirs.iter().map(|dir| machine.dir(dir)).take(1).collect(),
                volta: None,
            })
        );
        let npm = machine.dir(&format!("{prefix}/bin/npm"));
        let bin_npm = Unbound::Foreign(Layout::NodePrefix, OsString::from("bin/npm"));
        let refused = Reach::Unbindable("npm", machine.dir(prefix), bin_npm);
        for target in [second, PathBuf::from(format!("../../{leaf}-links/npm"))] {
            std::fs::remove_file(&npm).unwrap();
            symlink(&target, &npm).unwrap();
            assert_eq!(machine.reach("npm", &dirs).as_ref(), Some(&refused));
        }
    }
}

/// A root whose entries cannot be listed, a node prefix whose `bin`
/// cannot, and one with no `lib/node_modules` are refused, not bound:
/// `init` cannot tell what they hold.
#[test]
fn a_root_that_cannot_be_listed_is_refused() {
    let machine = Machine::new();
    machine.linked_prefix("opt/node");
    let node = machine.dir("opt/node");
    let listable = |dir: &Path, mode| {
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(mode)).unwrap();
    };
    let reach = |dir: &Path| {
        listable(dir, 0o311);
        let refused = machine.reach("npm", &["opt/node/bin"]);
        listable(dir, 0o755);
        refused
    };
    let unreadable = Some(Reach::Unbindable("npm", node.clone(), Unbound::Unreadable));
    assert_eq!(reach(&node.join("bin")), unreadable);
    machine.file("srv/share/npm/npm-cli.js");
    machine.link("srv/bin/npm", "../share/npm/npm-cli.js");
    let srv = machine.dir("srv");
    assert_eq!(
        machine.reach("npm", &["srv/bin"]),
        Some(Reach::Unbindable("npm", srv, Unbound::Unreadable))
    );
    let refused = reach(&node).unwrap();
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
/// `PATH` directory, the root, what is bound under it (`""` the root
/// whole), the binaries and links planted, and every top-level entry the
/// layout names. Each runner resolves outside its `PATH` directory, as a
/// file standing in it is bound alone.
struct Planted {
    runner: &'static str,
    bin: &'static str,
    root: &'static str,
    binds: &'static [&'static str],
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
const PNPM_CJS: &str = "global/5/node_modules/pnpm/bin/pnpm.cjs";
const WHOLE: &[&str] = &[""];
const PREFIX_BINDS: &[&str] = &["bin", "lib/node_modules"];

const PLANTED: &[Planted] = &[
    Planted {
        runner: "npm",
        bin: "home/.volta/bin",
        root: "home/.volta",
        binds: WHOLE,
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
        binds: PREFIX_BINDS,
        layout: (Layout::NodePrefix, "a node prefix"),
        files: NODE_FILES,
        links: &[NVM_LINK],
        entries: PREFIX,
    },
    Planted {
        runner: "npm",
        bin: "home/.local/share/fnm/node-versions/v20.11.0/installation/bin",
        root: "home/.local/share/fnm/node-versions/v20.11.0/installation",
        binds: PREFIX_BINDS,
        layout: (Layout::NodePrefix, "a node prefix"),
        files: NODE_FILES,
        links: &[NVM_LINK],
        entries: PREFIX,
    },
    Planted {
        runner: "bun",
        bin: "home/.bun/bin",
        root: "home/.bun",
        binds: WHOLE,
        layout: (Layout::Bun, "bun's root"),
        files: &["install/bun", "install/global/package.json"],
        links: &[("bin/bun", "../install/bun"), ("bin/bunx", "bun")],
        entries: &["_bun", "bin", "install"],
    },
    Planted {
        runner: "pnpm",
        bin: "home/.local/share/pnpm",
        root: "home/.local/share/pnpm",
        binds: WHOLE,
        layout: (Layout::PnpmHome, "pnpm's home"),
        files: &[PNPM_CJS, "nodejs/20.11.0/bin/node"],
        links: &[("pnpm", PNPM_CJS), ("node", "nodejs/20.11.0/bin/node")],
        entries: &[
            "global", "node", "nodejs", "npm", "npx", "pnpm", "pnpx", "store",
        ],
    },
];

/// Each recognised layout, planted with exactly the entries it names,
/// binds its root whole, or a node prefix its `bin` and `lib/node_modules`;
/// one `credentials` beside them and the root is refused, naming it. Each
/// tool's home stands at its default, `~/.local/share/pnpm` being pnpm's
/// on Linux, as [`Machine::homes`] names them.
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
        let spell = |under: &&str| at(under).trim_end_matches('/').replacen("home/", "~/", 1);
        let spelled: Vec<String> = planted.binds.iter().map(spell).collect();
        assert_eq!(binds, &spelled, "{}", planted.root);
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

/// An `npm` this user may not run earlier on `PATH`, its own file at mode
/// 0641, is passed over for the executable one after it, as the shell's
/// lookup passes it; with only that one, npm is not found. The kernel
/// grants a privileged process execute on any file with an execute bit,
/// so as root the 0641 one is found instead, as composite/tests.rs
/// asserts; the detector is a mode-0000 file only root can read.
#[test]
fn a_runner_that_cannot_run_is_passed_over_on_path() {
    let machine = Machine::new();
    machine.file_of("home/bin/npm", 0o641);
    machine.file("opt/node/bin/npm");
    machine.file("opt/node/bin/node");
    let sealed = machine.file_of("sealed", 0o000);
    let privileged = std::fs::read(sealed).is_ok();
    // As root the 0641 npm is found alone, and then the `node` it runs on
    // is what is missing.
    let (npm, missing) = match privileged {
        true => ("~/bin/npm".to_string(), "node"),
        false => (machine.spelled("opt/node/bin/npm"), "npm"),
    };
    let dirs = ["home/bin", "opt/node/bin"];
    let shadowed = machine.reach("npm", &dirs).unwrap();
    let Reach::Found { binds, .. } = shadowed else {
        panic!("an npm on PATH is found: {shadowed:?}");
    };
    assert_eq!(binds, [npm, machine.spelled("opt/node/bin/node")]);
    assert_eq!(
        machine.reach("npm", &["home/bin"]),
        Some(Reach::Missing(missing))
    );
}

/// A binary whose directory and file stand under a toolchain root every
/// box reads adds no bind, only its directory to PATH.
#[test]
fn a_runner_every_box_reads_adds_no_bind() {
    let mut found = Found::default();
    let system = Install {
        dir: PathBuf::from("/usr/bin"),
        entry: PathBuf::from("/usr/bin/npm"),
        real: PathBuf::from("/usr/share/nodejs/npm/bin/npm-cli.js"),
    };
    let homes = Homes::named(Path::new("/home/someone"), "linux", |_| None);
    assert_eq!(found.add(system, &homes), Ok(()));
    assert_eq!(
        (found.binds, found.path, found.volta),
        (Vec::<PathBuf>::new(), vec![PathBuf::from("/usr/bin")], None)
    );
}

/// A stack with no node-family runner resolves nothing.
#[test]
fn a_stack_with_no_node_runner_resolves_nothing() {
    let machine = Machine::new();
    let runners: Vec<Tool> = ["cargo", "make"].into_iter().flat_map(tools_for).collect();
    let path = machine.dir("bin").into_os_string();
    assert_eq!(reach(&runners, &path, &machine.homes(&[])), None);
}

/// The chief's reproductions of #717's third hold, A and B (outside HOME
/// and under it), E and F: pnpm's shape in a directory no tool names as
/// its home, its `node` linking into a sibling node prefix (named `node`,
/// `nodejs` or `other`) whose `share` holds a keystore. No ancestor is
/// climbed to: each is refused at its `PATH` directory, naming the file
/// pnpm resolves to, and verify fails closed with that sentence.
#[test]
fn a_directory_no_tool_names_is_refused_naming_what_it_resolves_to() {
    let machine = Machine::new();
    let shapes = [
        ("opt/tools", "node"),
        ("home/tools", "node"),
        ("opt/e", "nodejs"),
        ("opt/f", "other"),
    ];
    for (base, sibling) in shapes {
        machine.file(&format!("{base}/pnpm/global/pnpm.cjs"));
        machine.link(&format!("{base}/pnpm/pnpm"), "global/pnpm.cjs");
        machine.link(
            &format!("{base}/pnpm/node"),
            &format!("../{sibling}/bin/node"),
        );
        machine.linked_prefix(&format!("{base}/{sibling}"));
        machine.file(&format!("{base}/{sibling}/share/keyrings/user.keystore"));
        let pnpm = machine.dir(&format!("{base}/pnpm"));
        let cjs = pnpm.join("global/pnpm.cjs");
        let refused = machine.reach("pnpm", &[&format!("{base}/pnpm")]).unwrap();
        let unowned = Unbound::Unowned(cjs.clone());
        assert_eq!(refused, Reach::Unbindable("pnpm", pnpm.clone(), unowned));
        let which = format!(
            "where it resolves to {}, in no home its tool names and no node prefix, and which",
            cjs.display()
        );
        fails_closed(&refused, sentence("pnpm", &pnpm, &which));
    }
}

/// The chief's reproductions V and U, and A with pnpm's `node` in its own
/// home, outside HOME and under it. Named by no variable, V's and U's
/// directory is a node prefix holding `tools` or `install`, and refused
/// naming it. Named by `VOLTA_HOME`, `BUN_INSTALL` or `PNPM_HOME`, each
/// binds exactly that home: its named entries' contents are the tool's own
/// at the operator's bar, V's `tools/aws.creds` and U's
/// `install/token.txt` included. A pnpm home whose `node` links into a
/// node prefix beside it is refused, naming the `node` it resolves to.
#[test]
fn a_home_its_tool_names_binds_exactly_that_home() {
    let machine = Machine::new();
    for base in ["opt", "home"] {
        let (v, u, a) = (
            format!("{base}/v"),
            format!("{base}/u"),
            format!("{base}/a/pnpm"),
        );
        machine.file(&format!("{v}/tools/volta/bin/volta-shim"));
        machine.file(&format!("{v}/tools/aws.creds"));
        for shim in ["npm", "node"] {
            machine.link(&format!("{v}/bin/{shim}"), "../tools/volta/bin/volta-shim");
        }
        machine.file(&format!("{u}/install/bun-1.0/bun"));
        machine.file(&format!("{u}/install/token.txt"));
        machine.link(&format!("{u}/bin/bun"), "../install/bun-1.0/bun");
        let (v_bin, u_bin) = (format!("{v}/bin"), format!("{u}/bin"));
        let holds = |entry: &str| Unbound::Foreign(Layout::NodePrefix, OsString::from(entry));
        assert_eq!(
            machine.reach("npm", &[&v_bin]),
            Some(Reach::Unbindable("npm", machine.dir(&v), holds("tools")))
        );
        assert_eq!(
            machine.reach("bun", &[&u_bin]),
            Some(Reach::Unbindable("bun", machine.dir(&u), holds("install")))
        );
        let named = [("VOLTA_HOME", machine.var(&v))];
        assert_eq!(
            machine.reach_named("npm", &[&v_bin], &named),
            Some(Reach::Found {
                binds: vec![machine.spell(&v)],
                path: vec![machine.dir(&v_bin)],
                volta: Some(machine.dir(&v)),
            })
        );
        let named = [("BUN_INSTALL", machine.var(&u))];
        assert_eq!(
            machine.reach_named("bun", &[&u_bin], &named),
            Some(Reach::Found {
                binds: vec![machine.spell(&u)],
                path: vec![machine.dir(&u_bin)],
                volta: None,
            })
        );
        machine.file(&format!("{a}/global/pnpm.cjs"));
        machine.link(&format!("{a}/pnpm"), "global/pnpm.cjs");
        machine.link(&format!("{a}/node"), "../node/bin/node");
        machine.linked_prefix(&format!("{base}/a/node"));
        let named = [("PNPM_HOME", machine.var(&a))];
        let node = machine.dir(&format!("{base}/a/node/bin/node"));
        let refused = machine.reach_named("pnpm", &[&a], &named).unwrap();
        let outside = Unbound::Outside(node.clone());
        assert_eq!(refused, Reach::Unbindable("node", machine.dir(&a), outside));
        let which = format!("which it resolves out of, to {}, and which", node.display());
        fails_closed(&refused, sentence("node", &machine.dir(&a), &which));
        std::fs::remove_file(machine.dir(&format!("{a}/node"))).unwrap();
        machine.file(&format!("{a}/nodejs/20.11.0/bin/node"));
        machine.link(&format!("{a}/node"), "nodejs/20.11.0/bin/node");
        assert_eq!(
            machine.reach_named("pnpm", &[&a], &named),
            Some(Reach::Found {
                binds: vec![machine.spell(&a)],
                path: vec![machine.dir(&a)],
                volta: None,
            })
        );
    }
}

/// A volta shim in `~/.volta/bin` resolving out of `~/.volta` is refused,
/// naming the file it resolves to: a home is bound only where it holds the
/// file `PATH` found.
#[test]
fn a_shim_resolving_out_of_its_home_is_refused() {
    let machine = Machine::new();
    let shim = machine.file("opt/elsewhere/volta-shim");
    for name in ["npm", "node"] {
        machine.link(&format!("home/.volta/bin/{name}"), shim.to_str().unwrap());
    }
    let volta = machine.home().join(".volta");
    let refused = machine.reach("npm", &["home/.volta/bin"]).unwrap();
    let outside = Unbound::Outside(shim.clone());
    assert_eq!(refused, Reach::Unbindable("npm", volta.clone(), outside));
    let which = format!("which it resolves out of, to {}, and which", shim.display());
    fails_closed(&refused, sentence("npm", &volta, &which));
}

/// Each tool's home is the one its variable names, else its default,
/// pnpm's under `$XDG_DATA_HOME` or `~/.local/share` on Linux and under
/// `~/Library` on macOS. An empty variable is unset; one naming a relative
/// path, or a home that does not exist, names none; a home named through
/// a link is its canonical directory.
#[test]
fn each_tool_names_its_home_by_its_variable_else_its_default() {
    let machine = Machine::new();
    let planted = [
        "home/.volta",
        "home/.bun",
        "home/.local/share/pnpm",
        "home/Library/pnpm",
        "opt/v",
        "opt/b",
        "opt/p",
        "opt/data/pnpm",
    ];
    for dir in planted {
        std::fs::create_dir_all(machine.dir(dir)).unwrap();
    }
    machine.link("opt/plink", "p");
    let home = machine.home();
    let defaults = vec![
        (Layout::Volta, home.join(".volta")),
        (Layout::Bun, home.join(".bun")),
        (Layout::PnpmHome, home.join(".local/share/pnpm")),
    ];
    assert_eq!(machine.homes(&[]).tools, defaults);
    let macos = Homes::named(&home, "macos", |_| None).tools;
    assert_eq!(macos[2], (Layout::PnpmHome, home.join("Library/pnpm")));
    let data = [("XDG_DATA_HOME", machine.var("opt/data"))];
    let pnpm = (Layout::PnpmHome, machine.dir("opt/data/pnpm"));
    assert_eq!(machine.homes(&data).tools[2], pnpm);
    let set = [
        ("VOLTA_HOME", machine.var("opt/v")),
        ("BUN_INSTALL", machine.var("opt/b")),
        ("PNPM_HOME", machine.var("opt/plink")),
    ];
    let named = vec![
        (Layout::Volta, machine.dir("opt/v")),
        (Layout::Bun, machine.dir("opt/b")),
        (Layout::PnpmHome, machine.dir("opt/p")),
    ];
    assert_eq!(machine.homes(&set).tools, named);
    let unnamed = [
        // `.` stands wherever the test runs; read relative, it would name
        // that directory.
        ("VOLTA_HOME", OsString::from(".")),
        ("BUN_INSTALL", OsString::new()),
        ("PNPM_HOME", machine.var("opt/absent")),
    ];
    let bun = vec![(Layout::Bun, home.join(".bun"))];
    assert_eq!(machine.homes(&unnamed).tools, bun);
}

/// The adversarial position's reproduction: npm and node in
/// `opt/node:20/bin`, reached through the ASCII alias `opt/node20`. The
/// canonical directory the verifier would export splits at the `:`, so it
/// is refused naming it, and verify fails closed with init's sentence; so
/// is a root whose own name holds a newline, or a byte that is not UTF-8,
/// at the `PATH` directory below it.
#[test]
fn a_path_the_verifier_cannot_carry_is_refused() {
    let machine = Machine::new();
    machine.file("opt/node:20/bin/npm");
    machine.file("opt/node:20/bin/node");
    machine.link("opt/node20", "node:20");
    let bin = machine.dir("opt/node:20/bin");
    let refused = machine.reach("npm", &["opt/node20/bin"]).unwrap();
    let colon = Unbound::Uncarried(Flaw::Colon);
    assert_eq!(refused, Reach::Unbindable("npm", bin.clone(), colon));
    let which = "whose ':' would split the verifier's PATH, and which";
    fails_closed(&refused, sentence("npm", &bin, which));
    let flawed: [(&[u8], Flaw, &str); 2] = [
        (
            b"/srv/no\nde/bin",
            Flaw::Newline,
            "whose newline the verifier's lines cannot carry, and which",
        ),
        (
            b"/srv/n\xffde/bin",
            Flaw::NotUtf8,
            "which is not UTF-8, as bundle.json must be, and which",
        ),
    ];
    for (name, flaw, which) in flawed {
        let dir = PathBuf::from(OsStr::from_bytes(name));
        let (entry, real) = (dir.join("npm"), dir.join("npm"));
        let install = Install {
            dir: dir.clone(),
            entry,
            real,
        };
        let Err((root, unbound)) = Found::default().add(install, &nowhere()) else {
            panic!("{dir:?} is carried");
        };
        assert_eq!((&root, &unbound), (&dir, &Unbound::Uncarried(flaw)));
        let refused = Reach::Unbindable("npm", root, unbound);
        assert_eq!(refused.refusal(), Some(sentence("npm", &dir, which).0));
    }
}

/// A `'` in an install's path round-trips: the verifier's `export PATH=`,
/// run by bash, heads `PATH` with the directory exactly.
#[test]
fn an_install_path_holding_a_quote_round_trips_through_the_verifier() {
    let machine = Machine::new();
    machine.file("opt/it's/bin/npm");
    machine.file("opt/it's/bin/node");
    let found = machine.reach("npm", &["opt/it's/bin"]).unwrap();
    let ran = std::process::Command::new("bash")
        .arg("-c")
        .arg(format!("{}printf '%s' \"$PATH\"", found.script()))
        .env("PATH", "/usr/bin:/bin")
        .output()
        .unwrap();
    let bin = machine.spelled("opt/it's/bin");
    let path = String::from_utf8(ran.stdout).unwrap();
    assert_eq!(path, format!("{bin}:/usr/bin:/bin"));
}
