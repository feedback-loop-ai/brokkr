//! U8a2b (MB3, CR2; D5, D6): every managed writer kept from every admitted
//! MCP server's inputs, on a coordination root of each test's own. Before
//! its surveys an attempt's lease is extended to its fixed inputs, and once
//! they answer to its program trees ([`extended`]); every later admission
//! compares its writers with them ([`unserved`]). For a program source and
//! a system source alike — server first, writer first, the two started
//! together, an uncertain settlement, and disjoint worktrees side by side
//! (an engine killed with its seat surviving is a real dispatch's, in
//! `brokered_tests::writers_tests`) — each conflict refuses
//! with MB3's identity cause, never CR2's artifact-root cause, before
//! anything of the refused attempt is sealed, the source's bytes unchanged.
//! A bind of a source's subtree and a source bound beneath a writable root
//! are shown by a mount table each test injects ([`Shown`]), which no test
//! here can mount; every other test reads this namespace's own. Linux only:
//! no other host reads a mount table (decision 0063). These suites need
//! the library's own test seams (an injected root, its seal hooks, a held
//! lease), which `tests/capability_broker_launch.rs` and
//! `tests/capability_ledger.rs` cannot reach while the compile fence
//! stands, so the matrix lives here; the evidence maps each requirement.

use std::ffi::{OsStr, OsString};
use std::process::Command;

use brokkr_protocol::hands::{ServerBox, SERVER_RESOLVER};

use super::*;
use crate::engine::broker::inventory::Writing;
use crate::engine::broker::sources::records_kept;
use crate::engine::broker::sources::{extended, extended_with, fixed, fixed_with, kept};
use crate::engine::broker::sources::{launchers, surveying, Unroutable, HOPS_MAX, SYSTEM_ROOTS};
use crate::engine::broker::sources::{launches, launches_with, read, routed, Served};

thread_local! {
    /// The mount table a test shows this thread's roots, where it shows one.
    static SHOWN: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// A mount table shown to this thread's roots until dropped, on return or
/// unwind: the root mount of device `8:1` at `/`, then `lines`, each
/// `<id> <root> <point>` of that device.
struct Shown;

impl Shown {
    fn with(lines: &[(u64, &Path, &Path)]) -> Shown {
        let mut table = String::from("1 0 8:1 / / rw - ext4 /dev/root rw\n");
        for (id, root, point) in lines {
            let (root, point) = (root.display(), point.display());
            table.push_str(&format!(
                "{id} 1 8:1 {root} {point} rw - ext4 /dev/root rw\n"
            ));
        }
        Shown::text(table)
    }

    fn text(table: String) -> Shown {
        SHOWN.set(Some(table));
        Shown
    }
}

impl Drop for Shown {
    fn drop(&mut self) {
        SHOWN.set(None);
    }
}

/// How every root a test opens observes the mount table: as this thread's
/// test shows it, each path on the mount whose point is its longest
/// ancestor; otherwise as production reads this namespace's.
pub(in crate::engine::broker::admission) fn shown(
    budget: &Budget,
    served: &Path,
) -> Result<crate::engine::broker::sources::Table, Unknown> {
    let Some(table) = SHOWN.with_borrow(Clone::clone) else {
        return crate::engine::broker::sources::observed(budget, served);
    };
    let points: Vec<(PathBuf, u64)> = (table.lines())
        .filter_map(|line| {
            let fields: Vec<&str> = line.split(' ').collect();
            Some((PathBuf::from(fields.get(4)?), fields[0].parse().ok()?))
        })
        .collect();
    let on = move |path: &Path| {
        let under = points.iter().filter(|(point, _)| path.starts_with(point));
        under
            .max_by_key(|(point, _)| point.components().count())
            .map(|(_, id)| *id)
    };
    read(&mut table.as_bytes(), budget, served, Box::new(on))
}

/// A server's package beside the HOME, its one program written, canonical:
/// the source an attempt's server box is built from; and the server's
/// launch toward that program ([`launch`]), by a link in `srv/links`.
fn package(host: &Host) -> PathBuf {
    let root = host.dir.path().join("opt/docs");
    std::fs::create_dir_all(root.join("bin")).unwrap();
    std::fs::write(root.join("bin/server"), "#!/bin/sh\n").unwrap();
    let root = std::fs::canonicalize(root).unwrap();
    let links = plain(host, "srv/links");
    std::os::unix::fs::symlink(root.join("bin/server"), links.join("server")).unwrap();
    alias(host, "launch", Path::new("srv/links/server"));
    root
}

/// The server's launch as its plan names it: `launch`, a link toward
/// `srv/links/server`, a link toward the package's program.
fn launch(host: &Host) -> PathBuf {
    host.dir.path().join("launch")
}

/// The program `package` holds, as its server would run it.
fn program(package: &Path) -> Vec<u8> {
    std::fs::read(package.join("bin/server")).unwrap()
}

/// The bootstrap beside the HOME every server box of these tests runs.
fn bootstrap(host: &Host) -> PathBuf {
    let bootstrap = host.dir.path().join("boot/brokkr");
    std::fs::create_dir_all(bootstrap.parent().unwrap()).unwrap();
    std::fs::write(&bootstrap, "brokkr").unwrap();
    bootstrap
}

/// A link named `name` beside the HOME, toward `target`: what a writer's
/// bind may name to reach it by another spelling.
fn alias(host: &Host, name: &str, target: &Path) -> PathBuf {
    let link = host.dir.path().join(name);
    std::os::unix::fs::symlink(target, &link).unwrap();
    link
}

/// A plain directory named `name` beside the HOME, made.
fn plain(host: &Host, name: &str) -> PathBuf {
    let dir = host.dir.path().join(name);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The inputs a server built from `package` reserves on `host`: the fixed
/// ones, then its program tree and its launch.
fn inputs(host: &Host, package: &Path) -> Vec<Served> {
    let mut inputs = fixed(&bootstrap(host));
    inputs.push(Served::Source(package.to_path_buf()));
    inputs.push(Served::Source(launch(host)));
    inputs
}

/// `held` extended to `serves` within five seconds: how its seals
/// committed.
fn extend(held: &Held, serves: Vec<Served>) -> Result<Commit, Unadmitted> {
    let commits = Cell::new(Commit::Synced);
    extended(held, serves, soon(), &commits).map(|()| commits.get())
}

/// The attempt `ids` of a zero-grant run on `worktree`, its one `work`
/// site boxed to the writable `roots`.
fn writing(worktree: &Path, ids: [&str; 2], roots: &[&Path]) -> Claim {
    claim(worktree, ids, Protects::Nothing, &[boxed("work", roots)])
}

/// The attempt `ids` on `worktree`, its one `work` site boxed to that
/// worktree, admitted, and its lease extended to the inputs of a server
/// built from `package`.
fn serving(host: &Host, worktree: &Path, ids: [&str; 2], package: &Path) -> Held {
    let held = host.admit(&writing(worktree, ids, &[worktree])).unwrap();
    assert_eq!(extend(&held, inputs(host, package)), Ok(Commit::Synced));
    held
}

/// The three kinds of source every case is proved for: a program source,
/// reached through a link toward the package; a system source, `/usr/bin`,
/// reached through a link toward it, which the operating-system root it
/// lies in refuses (D6); and a link on the launch's route, in `srv/links`,
/// reached through a link toward that directory, which holds neither the
/// launch as spelled nor the program it resolves to. Each the writable
/// alias, the input a refusal names, and the bytes that must stay as they
/// are.
fn aliases(host: &Host, docs: &Path) -> [(PathBuf, PathBuf, PathBuf); 3] {
    let system = first_root(Path::new("/usr/bin"), &[]).unwrap();
    let shell = std::fs::canonicalize("/bin/sh").unwrap();
    let links = host.dir.path().join("srv/links");
    [
        (
            alias(host, "docs", docs),
            docs.to_path_buf(),
            docs.join("bin/server"),
        ),
        (alias(host, "bin", Path::new("/usr/bin")), system, shell),
        (
            alias(host, "links", &links),
            links.join("server"),
            docs.join("bin/server"),
        ),
    ]
}

/// A server's fixed inputs are the bootstrap, every system support source
/// a server box binds and the resolver file, each as spelled; the
/// temporary directory its identity is generated beneath, for its own
/// entries; and then every candidate of the box launcher's discovery, in
/// order, whether or not it stands (`hands::ServerBox::launchers`). Each is
/// reserved by its route ([`routed`]).
#[test]
fn a_servers_fixed_inputs_are_the_bootstrap_the_system_and_the_resolver() {
    let host = Host::new();
    let boot = bootstrap(&host);
    let system = brokkr_protocol::hands::HOST_TOOLCHAIN_BINDS.iter();
    let system = system.chain([&SERVER_RESOLVER]).map(PathBuf::from);
    let mut expected = vec![Served::Source(boot.clone())];
    expected.extend(system.map(Served::Source));
    expected.push(Served::Entries(std::env::temp_dir()));
    let d = host.dir.path();
    let launchers = vec![d.join("early/bwrap"), d.join("bwrap")];
    let mut found = expected.clone();
    found.extend(launchers.iter().cloned().map(Served::Source));
    assert_eq!(fixed_with(&boot, launchers), found);
    assert_eq!(fixed_with(&boot, Vec::new()), expected);
    assert_eq!(fixed(&boot), fixed_with(&boot, ServerBox::launchers()));
}

/// Every candidate of each launch is reserved as its server box's lookup
/// selects among them (`hands::ServerProgram::candidates`), whether or not
/// it stands: a bare name in each directory of the server `PATH`, an
/// absolute one as spelled; a relative one holding a `/`, and an empty
/// argv, name none.
#[test]
fn every_candidate_of_a_launch_is_reserved() {
    let absent = "/opt/brokkr-u8a2b/server";
    let argvs = [
        vec!["sh".into()],
        vec![absent.into()],
        vec!["brokkr-u8a2b".into()],
        vec!["bin/server".into()],
        vec![],
    ];
    let candidates = launches(argvs.iter().map(Vec::as_slice));
    let on_path =
        |name: &str| ["/usr/local/bin", "/usr/bin", "/bin"].map(|dir| Path::new(dir).join(name));
    let mut expected: Vec<Served> = on_path("sh").into_iter().map(Served::Source).collect();
    expected.push(Served::Source(absent.into()));
    expected.extend(on_path("brokkr-u8a2b").map(Served::Source));
    assert_eq!(candidates, expected);
}

/// Run four's H1 and H2, the survey's side (D6, operator ruling
/// 2026-10-19): a survey starts with the engine's environment, the
/// dynamic loader's `LD_*` removed and its `PATH` exactly the directories
/// of the box launcher's candidates, in order, whatever `PATH` the engine
/// holds; off Linux, where no launcher is a candidate, its own `PATH`.
#[test]
fn a_survey_starts_without_the_loaders_environment_on_the_launchers_directories() {
    let pair = |name: &str, value: &str| (OsString::from(name), OsString::from(value));
    let inherited = [
        pair("LD_PRELOAD", "/srv/preload.so"),
        pair("PATH", "/opt/early:/usr/bin"),
        pair("HOME", "/home/operator"),
        pair("LD_LIBRARY_PATH", "/srv/lib"),
        pair("OLD_LD_PATH", "kept"),
    ];
    let candidates = [
        PathBuf::from("/usr/local/bin/bwrap"),
        PathBuf::from("/bin/bwrap"),
    ];
    let expected = vec![
        pair("HOME", "/home/operator"),
        pair("OLD_LD_PATH", "kept"),
        pair("PATH", "/usr/local/bin:/bin"),
    ];
    assert_eq!(surveying(inherited.clone(), &candidates), expected);
    let unlaunched = vec![
        pair("PATH", "/opt/early:/usr/bin"),
        pair("HOME", "/home/operator"),
        pair("OLD_LD_PATH", "kept"),
    ];
    assert_eq!(surveying(inherited, &[]), unlaunched);
    let parents = launchers()
        .into_iter()
        .map(|launcher| launcher.parent().unwrap().to_path_buf());
    let server = std::env::join_paths(parents).unwrap();
    assert_eq!(server, OsString::from("/usr/local/bin:/usr/bin:/bin"));
}

/// The operating system's own roots, as the operator's ruling of
/// 2026-10-19 names them (D6), in the order a refusal names the first:
/// written here, never read from [`SYSTEM_ROOTS`], so a root production
/// omits is still a case of the matrix that proves them.
const OPERATOR_ROOTS: &str =
    "/usr /bin /sbin /lib /lib32 /lib64 /libx32 /etc /dev /boot /proc /sys";

/// The first operating-system root ([`OPERATOR_ROOTS`], in order)
/// `writable` lies at, above or below as the host lays it out, the bind
/// mounts `binds` (each `(root, point)`) shown: each compared where it
/// resolves, a path that does not stand by its nearest ancestor that does,
/// and then, where it lies at or below the point of a bind, the longest,
/// where that bind's root does, so a directory bound from `/etc` lies
/// within `/etc` as its mount shows it.
fn first_root(writable: &Path, binds: &[(&Path, &Path)]) -> Option<PathBuf> {
    let laid = |path: &Path| {
        let standing =
            |at: &Path| Some((std::fs::canonicalize(at).ok()?, path.strip_prefix(at).ok()?));
        let (at, rest) = path.ancestors().find_map(standing).unwrap();
        let at = at.join(rest);
        let mut under: Vec<_> = binds
            .iter()
            .filter(|(_, point)| at.starts_with(point))
            .collect();
        under.sort_by_key(|(_, point)| point.components().count());
        match under.last() {
            Some((root, point)) => root.join(at.strip_prefix(point).unwrap()),
            None => at,
        }
    };
    let writes = laid(writable);
    let related = |root: &&str| {
        let root = laid(Path::new(root));
        writes.starts_with(&root) | root.starts_with(&writes)
    };
    OPERATOR_ROOTS.split(' ').find(related).map(PathBuf::from)
}

/// Run four's H2, under the operator's ruling of 2026-10-19 (D6): a writer
/// whose root lies at, above or below each operating-system root the
/// operator named ([`OPERATOR_ROOTS`]) — the loader, `/etc/ld.so.preload`,
/// system libraries, programs and device nodes among what they hold — is
/// refused by MB3's identity cause naming the first root it reaches: writer
/// first, by the lease's first extension, before any survey; server first,
/// before anything of it is sealed. Its reach is injected; nothing of the
/// roots is written. The mount table is shown, whatever this host's own
/// binds, as one filesystem with `/etc` bound at `sysalias` beside the
/// HOME, so each root's own case names it where it resolves, and a writer
/// within that bind, which no path or link leads into `/etc`, is named by
/// `/etc` as its mount shows it. A writer apart from them, beside the
/// server, is admitted, and the extension commits. Then production's roots
/// are the operator's, in order.
#[test]
fn a_writer_reaching_an_operating_system_root_is_refused_before_the_survey() {
    let host = Host::new();
    let (a, b) = (host.worktree("a"), host.worktree("b"));
    let boot = bootstrap(&host);
    let sysalias = plain(&host, "sysalias");
    let binds = [(Path::new("/etc"), sysalias.as_path())];
    let _shown = Shown::with(&[(2, binds[0].0, binds[0].1)]);
    let mut writers = vec![PathBuf::from("/"), sysalias.join("brokkr-u8a2b")];
    for root in OPERATOR_ROOTS.split(' ').map(PathBuf::from) {
        writers.extend([root.join("brokkr-u8a2b"), root]);
    }
    let held = host.admit(&writing(&a, ["run-a", "a1"], &[&a])).unwrap();
    assert_eq!(extend(&held, fixed(&boot)), Ok(Commit::Synced));
    for (index, writable) in writers.iter().enumerate() {
        // `/` holds the coordination root too, which the door names first.
        let named = match writable.parent() {
            None => host.root(),
            Some(_) => first_root(writable, &binds).unwrap(),
        };
        let incoming = writing(&b, ["run-b", &format!("b{index}")], &[&b, writable]);
        assert_eq!(
            host.admit(&incoming),
            exposed("work", "run-b", &named),
            "{writable:?}"
        );
        assert!(
            !host.reservation(&b, "run-b").exists(),
            "nothing of it sealed"
        );
    }
    freed(&held);
    for (index, writable) in writers.iter().enumerate() {
        let named = first_root(writable, &binds).unwrap();
        let extension = writer_first(&host, pair(&host, index), writable, fixed(&boot));
        assert_eq!(
            extension,
            Err(exposing("work", "run-b", &named).into()),
            "{writable:?}"
        );
    }
    let apart = host.dir.path().join("srv");
    let extension = writer_first(&host, pair(&host, writers.len()), &apart, fixed(&boot));
    assert_eq!(extension, Ok(Commit::Synced));
    let operators: Vec<&str> = OPERATOR_ROOTS.split(' ').collect();
    assert_eq!(SYSTEM_ROOTS.to_vec(), operators);
}

/// Links named `links`, each `(name, target)`, `name` beside the HOME.
fn linked(host: &Host, links: &[(&str, &str)]) {
    for (name, target) in links {
        let link = host.dir.path().join(name);
        std::fs::create_dir_all(link.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(target, link).unwrap();
    }
}

/// A file named `name` beside the HOME, written, its directories made.
fn written(host: &Host, name: &str) -> PathBuf {
    let file = host.dir.path().join(name);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, "brokkr").unwrap();
    file
}

/// An input's route holds every symlink met on its way, one component at
/// a time, each by the path it lies at — a resolver-like chain of a
/// relative and an absolute link, a directory link above the input
/// (`lib -> usr/lib`), an alternatives chain (`usr/bin/x ->
/// ../../etc/alternatives/x -> ./../../usr/lib/x/libc`) — then where it
/// resolves, as the input it is, each once; a route that meets a component
/// that does not stand is reserved there and walked no further, `..` and
/// all. Run five's L1: `srv/a/missing/../../x` is reserved at
/// `srv/a/missing`, never at `srv/x`, so a writer confined to `srv/a`,
/// which could make `missing` a link leading it to `srv/a/x`, is refused
/// by MB3's identity cause naming it.
#[test]
fn every_link_on_an_inputs_route_is_reserved() {
    let host = Host::new();
    let d = host.dir.path();
    let resolver = written(&host, "opt/resolver/resolv.conf");
    let libc = written(&host, "usr/lib/x/libc");
    let climbing = plain(&host, "srv/a");
    let absolute = resolver.display().to_string();
    linked(
        &host,
        &[
            ("etc/resolv.conf", "../srv/links/resolver"),
            ("srv/links/resolver", &absolute),
            ("lib", "usr/lib"),
            ("usr/bin/x", "../../etc/alternatives/x"),
            ("etc/alternatives/x", "./../../usr/lib/x/libc"),
            ("dangling", "missing/../opt"),
        ],
    );
    let inputs = [
        Served::Source(d.join("etc/resolv.conf")),
        Served::Source(d.join("lib/x/libc")),
        Served::Source(d.join("usr/bin/x")),
        Served::Entries(d.join("dangling")),
        // A link given as an input is routed as a source: all met above.
        Served::Link(d.join("lib/x/libc")),
        Served::Source(climbing.join("missing/../../x")),
    ];
    let link = |name: &str| Served::Link(d.join(name));
    let missing = climbing.join("missing");
    let expected = vec![
        link("etc/resolv.conf"),
        link("srv/links/resolver"),
        Served::Source(resolver),
        link("lib"),
        Served::Source(libc),
        link("usr/bin/x"),
        link("etc/alternatives/x"),
        link("dangling"),
        Served::Entries(d.join("missing")),
        Served::Source(missing.clone()),
    ];
    let budget = Budget::new(soon());
    assert_eq!(routed(&inputs, &budget), Ok(expected.clone()));
    let writer = boxed("work", &[&climbing]);
    let compared = kept(
        [("run-b", &Writing::from(&writer))],
        &expected,
        &host.root,
        &budget,
    );
    assert_eq!(compared, Err(exposing("work", "run-b", &missing)));
}

/// A route is never reserved short: one of D5's [`HOPS_MAX`] symlinks is
/// walked whole, one more refuses by MB3's identity cause naming the input
/// and the hops, as do a route that is not absolute — a relative or an
/// empty spelling, as a relative or empty `PATH` entry or a relative
/// `TMPDIR` gives — and one that cannot be read, each by its own cause; a
/// deadline spent before a component is read refuses by MB3's deadline
/// cause. An extension whose route refuses seals nothing.
#[test]
fn a_route_past_its_bound_or_unread_refuses_and_reserves_nothing_less() {
    let host = Host::new();
    let d = host.dir.path();
    let file = written(&host, "hops/file");
    let chain = |name: &str, links: usize| {
        for hop in 0..links {
            let next = if hop + 1 == links {
                "file".to_string()
            } else {
                format!("{name}{}", hop + 1)
            };
            linked(&host, &[(&format!("hops/{name}{hop}"), &next)]);
        }
        Served::Source(d.join(format!("hops/{name}0")))
    };
    let budget = Budget::new(soon());
    let bounded = routed(&[chain("a", HOPS_MAX)], &budget).unwrap();
    assert_eq!(bounded.len(), HOPS_MAX + 1, "forty links and the file");
    assert_eq!(bounded.last(), Some(&Served::Source(file.clone())));
    let unrouted = |path: PathBuf, why| Err(Unknown::Unrouted(path, why));
    let over = d.join("hops/b0");
    let hops = unrouted(over.clone(), Unroutable::Hops);
    assert_eq!(routed(&[chain("b", HOPS_MAX + 1)], &budget), hops);
    for spelled in ["hops/file", ""] {
        let relative = Served::Entries(spelled.into());
        let refused = unrouted(spelled.into(), Unroutable::Relative);
        assert_eq!(routed(&[relative], &budget), refused, "{spelled:?}");
    }
    let unread = file.join("below");
    let below = Served::Source(unread.clone());
    assert_eq!(
        routed(&[below], &budget),
        unrouted(unread, Unroutable::Unread)
    );
    let spent = Budget::new(Instant::now());
    let source = Served::Source(file);
    assert_eq!(routed(&[source], &spent), Err(Unknown::Expired));
    let a = host.worktree("a");
    let held = host.admit(&writing(&a, ["run-a", "a1"], &[&a])).unwrap();
    let sealed = std::fs::read(host.reservation(&a, "run-a")).unwrap();
    assert_eq!(
        extend(&held, fixed(&over)),
        Err(Unknown::Unrouted(over, Unroutable::Hops).into())
    );
    let now = std::fs::read(host.reservation(&a, "run-a")).unwrap();
    assert_eq!(now, sealed, "nothing of it sealed");
}

/// The run-three counterexample, for three shapes of route of a fixed
/// input — a chain of file links, a directory link toward a directory
/// link, an alternatives chain — each through a directory that holds
/// neither the bootstrap as spelled nor where it resolves, nor any
/// ancestor of either: a writer confined to that directory is refused by
/// MB3's identity cause naming the link it could replace, server first,
/// and refuses the extension that precedes any survey, writer first; one
/// confined to a directory beside it is admitted and extended beside.
#[test]
fn a_writer_confined_to_a_link_on_a_fixed_route_is_refused_in_either_order() {
    let host = Host::new();
    let d = host.dir.path().to_path_buf();
    let target = written(&host, "opt/brokkr/brokkr").display().to_string();
    written(&host, "usr/lib/brokkr");
    linked(
        &host,
        &[
            ("boot1", "srv/links/brokkr"),
            ("srv/links/brokkr", &target),
            ("lib", "srv/links/lib"),
            ("srv/links/lib", "../../usr/lib"),
            ("usr/bin/brokkr", "../../etc/alternatives/brokkr"),
            ("etc/alternatives/brokkr", &target),
        ],
    );
    let other = plain(&host, "srv/other");
    let cases = [
        ("boot1", "srv/links", "srv/links/brokkr"),
        ("lib/brokkr", "srv/links", "srv/links/lib"),
        (
            "usr/bin/brokkr",
            "etc/alternatives",
            "etc/alternatives/brokkr",
        ),
    ];
    for (index, (input, writer, link)) in cases.into_iter().enumerate() {
        let (boot, writer, link) = (d.join(input), d.join(writer), d.join(link));
        let (a, b) = pair(&host, index);
        let held = host.admit(&writing(&a, ["run-a", "a1"], &[&a])).unwrap();
        assert_eq!(extend(&held, fixed(&boot)), Ok(Commit::Synced), "{input}");
        let incoming = writing(&b, ["run-b", "b1"], &[&b, &writer]);
        assert_eq!(host.admit(&incoming), exposed("work", "run-b", &link));
        freed(
            &host
                .admit(&writing(&b, ["run-b", "b2"], &[&b, &other]))
                .unwrap(),
        );
        freed(&held);
        let refused = Err(exposing("work", "run-b", &link).into());
        let first = writer_first(&host, (a.clone(), b.clone()), &writer, fixed(&boot));
        assert_eq!(first, refused, "{input}: writer first");
        let apart = writer_first(&host, (a, b), &other, fixed(&boot));
        assert_eq!(apart, Ok(Commit::Synced), "{input}: apart");
    }
}

/// A bind of the directory a link on a route lies in, at another path, is
/// shown by the mount table, not by any link: a writer whose root is such
/// a bind of `srv/links` could replace the launch's link there, and is
/// refused by MB3's identity cause naming it; one whose root binds a
/// directory beside it is kept from it.
#[test]
fn a_bind_of_the_directory_a_link_lies_in_is_refused() {
    let host = Host::new();
    let (b, _docs) = (host.worktree("b"), package(&host));
    let links = host.dir.path().join("srv/links");
    let (bound, beside) = (plain(&host, "bound"), plain(&host, "beside"));
    let serves = [Served::Link(links.join("server"))];
    for (source, refused) in [(&links, true), (&beside, false)] {
        let _shown = Shown::with(&[(2, source, &bound)]);
        let writer = boxed("work", &[&b, &bound]);
        let compared = kept(
            [("run-b", &Writing::from(&writer))],
            &serves,
            &host.root,
            &Budget::new(soon()),
        );
        let expected = refused.then(|| exposing("work", "run-b", &links.join("server")));
        assert_eq!(compared.err(), expected, "{source:?}");
    }
}

/// An extension keeps every input its attempt reserved before: a route
/// the first extension walked through a link, which no longer stands as
/// the next is walked, stays reserved, so a writer able to replace that
/// link is still refused.
#[test]
fn an_extension_keeps_every_input_reserved_before() {
    let host = Host::new();
    let (a, b) = pair(&host, 0);
    let boot = written(&host, "opt/brokkr/brokkr");
    let links = plain(&host, "srv/links");
    std::os::unix::fs::symlink(&boot, links.join("brokkr")).unwrap();
    let spelled = alias(&host, "boot1", &links.join("brokkr"));
    let held = host.admit(&writing(&a, ["run-a", "a1"], &[&a])).unwrap();
    assert_eq!(extend(&held, fixed(&spelled)), Ok(Commit::Synced));
    std::fs::remove_file(&spelled).unwrap();
    std::fs::write(&spelled, "brokkr").unwrap();
    assert_eq!(extend(&held, fixed(&spelled)), Ok(Commit::Synced));
    let incoming = writing(&b, ["run-b", "b1"], &[&b, &links]);
    assert_eq!(
        host.admit(&incoming),
        exposed("work", "run-b", &links.join("brokkr"))
    );
    freed(&held);
}

/// A link a standing writer swapped while the survey observed the launch,
/// and swapped back before the extension that follows it, still refuses
/// that extension: the route is walked as the extension reserves it, and
/// every link on it is reserved, so the writer that could replace one is
/// named whatever the survey saw.
#[test]
fn a_link_swapped_between_the_survey_and_the_extension_still_refuses() {
    let host = Host::new();
    let (docs, (a, b)) = (package(&host), pair(&host, 0));
    let links = host.dir.path().join("srv/links");
    let elsewhere = written(&host, "elsewhere/server");
    let standing = host.admit(&writing(&b, ["run-b", "b1"], &[&b, &links]));
    let held = host.admit(&writing(&a, ["run-a", "a1"], &[&a])).unwrap();
    let boot = fixed(&bootstrap(&host));
    assert_eq!(extend(&held, boot), Ok(Commit::Synced), "before the survey");
    let link = links.join("server");
    let swapped = |toward: &Path| {
        std::fs::remove_file(&link).unwrap();
        std::os::unix::fs::symlink(toward, &link).unwrap();
    };
    swapped(&elsewhere);
    let surveyed = std::fs::canonicalize(launch(&host)).unwrap();
    swapped(&docs.join("bin/server"));
    assert_eq!(surveyed, elsewhere, "the survey saw the swap");
    let programs = vec![Served::Source(launch(&host)), Served::Source(docs)];
    let refused = Err(exposing("work", "run-b", &link).into());
    assert_eq!(extend(&held, programs), refused);
    freed(&standing.unwrap());
    freed(&held);
}

/// Server first, for a program source and a system source: while one
/// worktree's attempt holds its server's inputs, a zero-grant run on
/// another worktree whose box could write one — by a link toward it, or,
/// for the package, a directory inside it or one holding it — is refused by
/// MB3's identity cause before anything of it is sealed, and one writing
/// only its own worktree is admitted beside it; the source is unchanged.
#[test]
fn a_writer_that_can_write_a_reserved_source_is_refused_before_dispatch() {
    let host = Host::new();
    let (a, b, docs) = (host.worktree("a"), host.worktree("b"), package(&host));
    let [program_case, system_case, hop_case] = aliases(&host, &docs);
    let before = program(&docs);
    let _serving = serving(&host, &a, ["run-a", "a1"], &docs);
    let cases = [
        (program_case.0, docs.clone()),
        (docs.join("bin"), docs.clone()),
        (host.dir.path().join("opt"), docs.clone()),
        (system_case.0, system_case.1),
        (hop_case.0, hop_case.1),
    ];
    for (index, (writable, served)) in cases.iter().enumerate() {
        let attempt = format!("b{index}");
        let incoming = writing(&b, ["run-b", &attempt], &[&b, writable]);
        let refused = exposed("work", "run-b", served);
        assert_eq!(host.admit(&incoming), refused, "{writable:?}");
        assert!(
            !host.reservation(&b, "run-b").exists(),
            "nothing of it sealed"
        );
    }
    freed(&host.admit(&writing(&b, ["run-b", "b9"], &[&b])).unwrap());
    assert_eq!(program(&docs), before);
}

/// Writer first, for a program source and a system source: a standing
/// zero-grant run whose box could write one refuses the extension of an
/// attempt whose server is built from it, before anything of that server
/// starts, its reservation left byte for byte as it stood; a disjoint
/// standing run does not.
#[test]
fn a_broker_whose_sources_a_standing_writer_can_write_is_refused_before_lookup() {
    let host = Host::new();
    let docs = package(&host);
    for (index, (writable, served, bytes)) in aliases(&host, &docs).iter().enumerate() {
        let before = std::fs::read(bytes).unwrap();
        let extension = writer_first(&host, pair(&host, index), writable, inputs(&host, &docs));
        let refused = Err(exposing("work", "run-b", served).into());
        assert_eq!(extension, refused, "{served:?}");
        assert_eq!(std::fs::read(bytes).unwrap(), before);
    }
    let (a, b) = (host.worktree("a"), host.worktree("b"));
    let _disjoint = host.admit(&writing(&b, ["run-b", "b1"], &[&b])).unwrap();
    freed(&serving(&host, &a, ["run-a", "a1"], &docs));
}

/// The worktrees `a<index>` and `b<index>` beside `host`'s HOME, made.
fn pair(host: &Host, index: usize) -> (PathBuf, PathBuf) {
    let named = |name: &str| host.worktree(&format!("{name}{index}"));
    (named("a"), named("b"))
}

/// Writer first: a zero-grant run on `b` standing, its record planted as a box
/// that could also write `writable` (the door keeps one that reaches the
/// coordination root from being admitted at all), then an attempt on `a`
/// admitted and its lease extended to `serves`: how the extension committed,
/// or why it was refused, its reservation then left byte for byte as it stood;
/// both settled after.
fn writer_first(
    host: &Host,
    (a, b): (PathBuf, PathBuf),
    writable: &Path,
    serves: Vec<Served>,
) -> Result<Commit, Unadmitted> {
    let standing = host.admit(&writing(&b, ["run-b", "w"], &[&b]));
    let reach = vec![boxed("work", &[&b, writable])];
    planted_writers(host, &b, ["run-b", "w"], reach);
    let held = host.admit(&writing(&a, ["run-a", "w"], &[&a])).unwrap();
    let sealed = std::fs::read(host.reservation(&a, "run-a")).unwrap();
    let extension = extend(&held, serves);
    if extension.is_err() {
        let now = std::fs::read(host.reservation(&a, "run-a")).unwrap();
        assert_eq!(now, sealed, "left as it stood");
    }
    freed(&standing.unwrap());
    freed(&held);
    extension
}

/// H1, the mount table showing `lines`: a writer whose root is `writable`
/// is refused by MB3's identity cause naming the package `docs`, server
/// first, and refuses the extension, writer first, where `conflicts`;
/// otherwise it is admitted beside the server, and the extension commits.
fn shown_in_either_order(
    host: &Host,
    (docs, index): (&Path, usize),
    lines: &[(u64, &Path, &Path)],
    writable: &Path,
    conflicts: bool,
) {
    let _shown = Shown::with(lines);
    let (a, b) = pair(host, index);
    let first = serving(host, &a, ["run-a", "a1"], docs);
    let incoming = host.admit(&writing(&b, ["run-b", "b1"], &[&b, writable]));
    let refused = exposing("work", "run-b", docs);
    let expected = conflicts.then(|| Unadmitted::Identity(refused.clone()));
    assert_eq!(incoming.as_ref().err(), expected.as_ref(), "server first");
    if let Ok(held) = &incoming {
        freed(held);
    }
    freed(&first);
    let extension = writer_first(host, (a, b), writable, inputs(host, docs));
    let expected = if conflicts {
        Err(refused.into())
    } else {
        Ok(Commit::Synced)
    };
    assert_eq!(extension, expected, "writer first");
}

/// Before any survey, the fixed inputs alone: a standing writer able to
/// write the bootstrap, the resolver file, or the temporary directory a
/// box's identity is generated beneath refuses the lease's first extension
/// naming it; one whose root lies below the temporary directory, as every
/// worktree here does, cannot replace its entries and is kept from it.
#[test]
fn every_fixed_input_is_reserved_before_the_survey() {
    let host = Host::new();
    // Outside the temporary directory, as an installed one lies; not
    // standing, it is reserved at its first missing component.
    let boot = PathBuf::from("/opt/brokkr-u8a2b/bin/brokkr");
    let temporary = crate::engine::broker::resolved(std::env::temp_dir());
    let missing = boot.ancestors().take_while(|at| !at.exists()).last();
    let missing = missing.unwrap().to_path_buf();
    let mut cases = vec![(boot.parent().unwrap().to_path_buf(), missing)];
    cases.push((temporary.clone(), temporary));
    if Path::new(SERVER_RESOLVER).exists() {
        // A root through a link toward the resolver writes where it
        // resolves, never a link on its way.
        let link = alias(&host, "resolver", Path::new(SERVER_RESOLVER));
        cases.push((link, std::fs::canonicalize(SERVER_RESOLVER).unwrap()));
    }
    for (index, (writable, served)) in cases.iter().enumerate() {
        // A resolver within `/etc` is the root's to refuse (D6).
        let served = first_root(writable, &[]).unwrap_or(served.clone());
        let extension = writer_first(&host, pair(&host, index), writable, fixed(&boot));
        assert_eq!(extension, Err(exposing("work", "run-b", &served).into()));
    }
}

/// H1: a bind of a source's subtree at another path is shown by the mount
/// table, not by any link: a writer whose root lies within such a bind of
/// the package's `bin` is refused, server first, and refuses the
/// extension, writer first, by MB3's identity cause; within the same bind
/// of a directory beside the package it is kept from it.
#[test]
fn a_bind_of_a_sources_subtree_is_refused_in_either_order() {
    let host = Host::new();
    let docs = package(&host);
    let (alias, beside) = (plain(&host, "srv/alias"), plain(&host, "beside"));
    let bound = plain(&host, "srv/alias/lib");
    for (index, (shown, conflicts)) in [(docs.join("bin"), true), (beside, false)]
        .into_iter()
        .enumerate()
    {
        let lines = [(2, shown.as_path(), alias.as_path())];
        shown_in_either_order(&host, (&docs, index), &lines, &bound, conflicts);
    }
    assert_eq!(program(&docs), b"#!/bin/sh\n");
}

/// H1: a writable root holding a bind of a source nested beneath it can
/// write that source, as the mount table shows: refused, server first,
/// and refusing the extension, writer first, by MB3's identity cause; a
/// root holding a bind of a directory beside the package is kept from it.
#[test]
fn a_source_bound_beneath_a_writable_root_is_refused_in_either_order() {
    let host = Host::new();
    let docs = package(&host);
    let (parent, beside) = (plain(&host, "srv/parent"), plain(&host, "beside"));
    let nested = parent.join("x");
    for (index, (shown, conflicts)) in [(docs.clone(), true), (beside, false)]
        .into_iter()
        .enumerate()
    {
        let lines = [(2, shown.as_path(), nested.as_path())];
        shown_in_either_order(&host, (&docs, index), &lines, &parent, conflicts);
    }
}

/// H1: a mount table that does not prove where a writer or a source lies
/// is never read as "no aliases": one that does not parse, names no mount
/// a path lies on, or names one whose root is no path refuses by MB3's
/// identity cause naming that path; one past D5's bound, or read past the
/// deadline, by MB3's bound and deadline causes. The temporary directory
/// bound beneath a writable root is shown too.
#[test]
fn a_mount_table_that_proves_nothing_keeps_no_writer() {
    let host = Host::new();
    let (b, docs) = (host.worktree("b"), package(&host));
    let writer = boxed("work", &[&b]);
    let serves = inputs(&host, &docs);
    let first = serves[0].path().to_path_buf();
    let compared = |table: String| {
        let _shown = Shown::text(table);
        kept(
            [("run-b", &Writing::from(&writer))],
            &serves,
            &host.root,
            &Budget::new(soon()),
        )
    };
    let unmounted = |path: &Path| Err(Unknown::Unmounted(path.to_path_buf()));
    assert_eq!(compared("not a mount table\n".into()), unmounted(&first));
    let elsewhere = format!("2 1 8:1 / {} rw - ext4 /dev/root rw\n", docs.display());
    assert_eq!(compared(elsewhere), unmounted(&b), "no mount it lies on");
    let foreign = "1 0 0:4 net:[4026531840] / rw - nsfs nsfs rw\n";
    assert_eq!(
        compared(foreign.into()),
        unmounted(&b),
        "a root that is no path"
    );
    let nested = format!(
        "1 0 8:1 / / rw - ext4 /dev/root rw\n2 1 0:4 net:[4026531840] {} rw - nsfs nsfs rw\n",
        b.join("ns").display()
    );
    assert_eq!(
        compared(nested),
        unmounted(&b),
        "a mount below a writable root whose root is no path"
    );
    assert_eq!(compared("\n".repeat(65_537)), Err(Unknown::Bounds));
    let spent = Budget::new(Instant::now());
    let read = read(&mut "".as_bytes(), &spent, &first, Box::new(|_| Some(1)));
    assert_eq!(read.err(), Some(Unknown::Expired));
    let temporary = crate::engine::broker::resolved(std::env::temp_dir());
    let parent = plain(&host, "srv/parent");
    let _shown = Shown::with(&[(2, &temporary, &parent.join("tmp"))]);
    let writer = boxed("work", &[&b, &parent]);
    let entries = [Served::Entries(temporary.clone())];
    let refused = Err(exposing("work", "run-b", &temporary));
    assert_eq!(
        kept(
            [("run-b", &Writing::from(&writer))],
            &entries,
            &host.root,
            &Budget::new(soon())
        ),
        refused
    );
}

/// A writer of the attempt's own reach — a zero-grant panel sibling, a
/// step's member — is a participant like any other: its box able to write
/// the package the attempt's server is built from refuses the extension
/// before any member starts.
#[test]
fn a_zero_grant_sibling_that_can_write_its_attempts_server_refuses_it() {
    let host = Host::new();
    let (a, docs) = (host.worktree("a"), package(&host));
    let link = alias(&host, "docs", &docs);
    let writers = [
        boxed("work:keeper", &[&a]),
        boxed("work:sibling", &[&a, &link]),
    ];
    let held = host
        .admit(&claim(&a, ["run-a", "a1"], Protects::Nothing, &writers))
        .unwrap();
    let refused = Err(exposing("work:sibling", "run-a", &docs).into());
    assert_eq!(extend(&held, inputs(&host, &docs)), refused);
}

/// A run's later attempt — a retry, or a fallback link's, which is
/// dispatched as the run's next attempt — is compared with the sources its
/// own earlier attempt, not proven settled, still holds: a writer of it
/// able to write them is refused, though the run's own lease admits it.
#[test]
fn a_runs_later_attempt_is_kept_from_its_earlier_attempts_sources() {
    let host = Host::new();
    let (a, docs) = (host.worktree("a"), package(&host));
    let link = alias(&host, "docs", &docs);
    let _first = serving(&host, &a, ["run-a", "a1"], &docs);
    let next = writing(&a, ["run-a", "a2"], &[&a, &link]);
    assert_eq!(host.admit(&next), exposed("work", "run-a", &docs));
    freed(&host.admit(&writing(&a, ["run-a", "a3"], &[&a])).unwrap());
}

/// Two started together admit at most one of a conflicting pair, in
/// either order, for a program source and a system source: an extension
/// that starts while a conflicting admission holds its comparison open
/// waits on the lock and is refused by it, and an admission that starts
/// while a conflicting extension holds its comparison open is refused by
/// the sources it sealed.
#[test]
fn simultaneous_starts_admit_at_most_one_of_a_conflicting_pair() {
    let host = Host::new();
    let docs = package(&host);
    for (index, (link, served, bytes)) in aliases(&host, &docs).iter().enumerate() {
        let before = std::fs::read(bytes).unwrap();
        raced(
            &host,
            pair(&host, index),
            link,
            &|| inputs(&host, &docs),
            served,
        );
        assert_eq!(std::fs::read(bytes).unwrap(), before);
    }
}

/// Two started together, in either order, the attempt on `a` extended to
/// `serves` and a zero-grant run on `b` whose box could also write
/// `writable`: an extension started while the admission holds its
/// comparison open waits on the lock and is refused naming `served`, and an
/// admission started while the extension holds its comparison open is
/// refused by what it sealed.
fn raced(
    host: &Host,
    (a, b): (PathBuf, PathBuf),
    writable: &Path,
    serves: &(dyn Fn() -> Vec<Served> + Sync),
    served: &Path,
) {
    let held = host.admit(&writing(&a, ["run-a", "a1"], &[&a])).unwrap();
    let incoming = writing(&b, ["run-b", "b1"], &[&b, writable]);
    let (admitted, extending) = std::thread::scope(|scope| {
        let (compared, told) = mpsc::channel();
        let root = &host.root;
        let holding = scope.spawn(|| admit_with(root, &incoming, soon(), &held_open(compared)));
        let racing = told.recv().map(|()| extend(&held, serves()));
        (holding.join().unwrap(), racing)
    });
    let writer = admitted.unwrap();
    assert_eq!(extending, Ok(Err(exposing("work", "run-b", served).into())));
    freed(&writer);
    let (extending, admitted) = std::thread::scope(|scope| {
        let (compared, told) = mpsc::channel();
        let holding = scope.spawn(|| {
            let (open, commits) = (held_open(compared), Cell::new(Commit::Synced));
            extended_with(&held, serves(), (soon(), &commits), &open)
        });
        let incoming = writing(&b, ["run-b", "b2"], &[&b, writable]);
        let racing = told.recv().map(|()| host.admit(&incoming).map(drop));
        (holding.join().unwrap(), racing)
    });
    assert_eq!(extending, Ok(()));
    let refused = Err(Unadmitted::Identity(exposing("work", "run-b", served)));
    assert_eq!(admitted, Ok(refused));
    freed(&held);
}

/// What holds a comparison open: it says so on `compared`, then waits.
fn held_open(compared: mpsc::Sender<()>) -> impl Fn() {
    move || {
        compared.send(()).unwrap();
        std::thread::sleep(Duration::from_millis(300));
    }
}

/// A free lock or an uncertain settlement releases no source, for a
/// program source and a system source: an extended lease whose engine never
/// released it keeps refusing a writer that could write its sources, the
/// lock held by no one, its reservation byte for byte as sealed; so does
/// one whose positive fact was sealed but which was never released, and one
/// whose driver's cleanup was witnessed unresolved. Only a release on
/// durable settlement frees them.
#[test]
fn a_free_lock_or_an_uncertain_settlement_releases_no_source() {
    let unresolved = Cleanup::Unresolved {
        reason: Unsettled::Stdout,
    };
    let ends: [&dyn Fn(Held); 4] = [
        &drop,
        &|held| assert_eq!(proved(&held, soon()), Ok(Commit::Synced)),
        &|held| witnessed_by(&mut Some(held), &unresolved, soon()),
        &|held| freed(&held),
    ];
    for (index, end) in ends.into_iter().enumerate() {
        let host = Host::new();
        let (b, docs) = (host.worktree("b"), package(&host));
        for (case, (link, served, bytes)) in aliases(&host, &docs).iter().enumerate() {
            let before = std::fs::read(bytes).unwrap();
            let (a, run) = (host.worktree(&format!("a{case}")), format!("run-a{case}"));
            end(serving(&host, &a, [&run, "a1"], &docs));
            let at = host.reservation(&a, &run);
            let sealed = std::fs::read(&at).unwrap();
            drop(taken(&host.root().join(LOCK)));
            let incoming = writing(&b, [&format!("run-b{case}"), "b1"], &[&b, link]);
            if index == 3 {
                // Released on durable settlement: its sources are free.
                freed(&host.admit(&incoming).unwrap());
                continue;
            }
            let refused = exposed("work", &format!("run-b{case}"), served);
            assert_eq!(host.admit(&incoming), refused, "{index}");
            assert_eq!(std::fs::read(&at).unwrap(), sealed, "{index}");
            assert_eq!(std::fs::read(bytes).unwrap(), before, "{index}");
            let reservation: Reservation = serde_json::from_slice(&sealed).unwrap();
            let reserved = &reservation.attempts["a1"].serves;
            assert!(
                reserved.contains(&Served::Source(docs.clone())),
                "{index}: still reserved"
            );
        }
    }
}

/// The one attempt of the run `run` of `worktree` standing in `home`'s
/// coordination root, its positive settlement fact sealed as the engine
/// that proved it settled seals it just before the release, which a crash
/// there leaves for the next admission to recover: the attempt.
pub(in crate::engine) fn proven_settled(home: &Path, worktree: &Path, run: &str) -> String {
    let repo = canonical::sha256_bytes(worktree.as_os_str().as_bytes());
    let at = coordination(home).join(&repo).join(run).join(RESERVATION);
    let reservation: Reservation = serde_json::from_slice(&std::fs::read(at).unwrap()).unwrap();
    let attempt = reservation.attempts.into_keys().next().unwrap();
    let held = Held {
        coordination: Arc::new(opened(home)),
        repo,
        run: run.into(),
        attempt: attempt.clone(),
        root: None,
        unproven: false,
        staked: false,
        unsynced: false,
        workdir: worktree.to_path_buf(),
        authored: Authored::default(),
        adopted: None,
    };
    assert_eq!(proved(&held, soon()), Ok(Commit::Synced));
    attempt
}

const BOUND_DIR: &str = "BROKKR_BOUND_DIR";

/// The child entry run in a mount namespace of its own.
const BOUND: &str = "engine::broker::admission::tests::sources::bound_engine";

/// Admission against binds really mounted, the mount table read as
/// production reads it: run by [`real_binds_of_a_source_are_refused_by_the_mount_table`]
/// in a mount namespace whose `bound` is a bind of the package's `bin`,
/// `nest/inner` a bind of the package, and `apart` a bind of a directory
/// beside it. Inert without [`BOUND_DIR`]; production reads no such
/// variable.
#[test]
fn bound_engine() {
    let Ok(dir) = std::env::var(BOUND_DIR) else {
        return;
    };
    let dir = PathBuf::from(dir);
    let root = Arc::new(opened(&dir.join("home")));
    let (a, b, docs) = (dir.join("a"), dir.join("b"), dir.join("opt/docs"));
    let held = admit(&root, &writing(&a, ["run-a", "a1"], &[&a]), soon()).unwrap();
    let mut serves = fixed(&dir.join("boot/brokkr"));
    serves.push(Served::Source(docs.clone()));
    assert_eq!(extend(&held, serves), Ok(Commit::Synced));
    let refused = Some(Unadmitted::Identity(exposing("work", "run-b", &docs)));
    for (index, writable) in [dir.join("bound/lib"), dir.join("nest")].iter().enumerate() {
        let incoming = writing(&b, ["run-b", &format!("b{index}")], &[&b, writable]);
        assert_eq!(
            admit(&root, &incoming, soon()).err(),
            refused,
            "{writable:?}"
        );
    }
    let apart = writing(&b, ["run-b", "b9"], &[&b, &dir.join("apart/lib")]);
    assert_eq!(admit(&root, &apart, soon()).map(drop), Ok(()));
    println!("bound: refused within a bind and beneath one, kept apart");
}

/// H1 in real mounts: in a user and mount namespace of its own, with the
/// package's `bin` bound at `bound` and the package bound at `nest/inner`,
/// a writer whose root lies within the first bind, or holds the second
/// beneath it, is refused by MB3's identity cause as the mount table and
/// `statx` show them, and one within a bind of a directory beside the
/// package is admitted. A host that cannot make such a namespace skips it
/// with its reason: host result pending.
#[test]
fn real_binds_of_a_source_are_refused_by_the_mount_table() {
    let host = Host::new();
    let docs = package(&host);
    bootstrap(&host);
    let made = ["a", "b", "bound", "nest/inner", "apart", "beside"].map(|name| plain(&host, name));
    let binds = [
        (docs.join("bin"), &made[2]),
        (docs.clone(), &made[3]),
        (made[5].clone(), &made[4]),
    ];
    // The whole host, these binds beside it, in namespaces of its own.
    let boxed = |bwrap: &Path, binds: &[(PathBuf, &PathBuf)], argv: &[&OsStr]| {
        let mut command = Command::new(bwrap);
        command.args(["--dev-bind", "/", "/"]);
        for (from, to) in binds {
            command.arg("--bind").arg(from).arg(to);
        }
        command
            .arg("--")
            .args(argv)
            .env(BOUND_DIR, host.dir.path())
            .output()
    };
    let bwrap = brokkr_protocol::hands::require_bwrap().unwrap_or_default();
    if !boxed(&bwrap, &[], &["true".as_ref()]).is_ok_and(|probed| probed.status.success()) {
        let required = brokkr_protocol::hands::boundary_evidence_required();
        let reason = "no mount namespace for real binds: host result pending";
        return brokkr_protocol::hands::skip_boundary_proof(required, reason);
    }
    let exe = std::env::current_exe().unwrap();
    let argv = [
        exe.as_os_str(),
        "--exact".as_ref(),
        BOUND.as_ref(),
        "--nocapture".as_ref(),
    ];
    let ran = boxed(&bwrap, &binds, &argv).unwrap();
    let said = String::from_utf8_lossy(&ran.stdout);
    assert!(ran.status.success(), "{said}");
    assert!(
        said.contains("bound: refused within a bind and beneath one, kept apart"),
        "{said}"
    );
    assert_eq!(program(&docs), b"#!/bin/sh\n");
}

/// A zero-grant run on `worktree`, its one `work` site boxed to it and to
/// `writable`, admitted in `home`'s coordination root and left standing:
/// a writer already running when a broker of another worktree starts.
pub(in crate::engine) fn standing_writer(home: &Path, worktree: &Path, writable: &Path) -> Held {
    let root = Arc::new(opened(home));
    let claim = writing(worktree, ["run-b", "b1"], &[worktree, writable]);
    admit(&root, &claim, soon()).unwrap()
}

/// The lease of `engine`'s run on its worktree for the attempt `attempt`,
/// its one `work` site boxed to the worktree, admitted in `home`'s
/// coordination root and held on the engine, as dispatch holds it.
pub(in crate::engine) fn leased(engine: &mut Engine, home: &Path, attempt: &str) {
    let worktree = std::fs::canonicalize(engine.workdir()).unwrap();
    let root = Arc::new(opened(home));
    let claim = writing(&worktree, [&engine.run_id.clone(), attempt], &[&worktree]);
    engine.held = Some(admit(&root, &claim, soon()).unwrap());
}

/// `engine`'s lease ([`leased`]) as one admitted with evidence at stake,
/// whose settlement the fence rule fails on a refusal.
pub(in crate::engine) fn staked(engine: &mut Engine) {
    engine.held.as_mut().expect("a lease").staked = true;
}

/// The next seal on this thread commits, but its last directory sync
/// fails, so it reports itself committed unsynced (L1).
pub(in crate::engine) fn unsynced_next() {
    LATE.set(Some(Late::FailFinal));
}

/// An incoming dispatch is refused before its first driver starts, for a
/// program source and a system source: with another worktree's server
/// holding its sources, a sequence whose last step's zero-grant panel
/// member could write one is refused by MB3's identity cause, the union
/// of its steps' reach compared before its first step, and no step's
/// driver starts.
#[test]
fn a_dispatch_whose_later_step_could_write_a_reserved_source_starts_nothing() {
    if !alone("sources::a_dispatch_whose_later_step_could_write_a_reserved_source_starts_nothing") {
        return;
    }
    let host = Host::new();
    let _home = Coordinated::at(&host.home);
    let (kept, docs) = (host.worktree("kept"), package(&host));
    let _serving = serving(&host, &kept, ["run-s", "s1"], &docs);
    for (link, served, bytes) in aliases(&host, &docs) {
        let before = std::fs::read(&bytes).unwrap();
        let (_dir, mut engine, ran) = sequenced(&link);
        let run = engine.run_id.clone();
        refused_with(
            &mut engine,
            exposing("work:finish:m1", &run, &served).into(),
        );
        assert!(!ran.exists(), "no step's driver started");
        assert_eq!(std::fs::read(&bytes).unwrap(), before);
    }
}

/// A writer outside a workspace box has no confined privilege, so a
/// server's write-exclusion proof does not hold against it: standing, it
/// refuses an extension; incoming, it is refused beside reserved sources;
/// with no source reserved it is admitted, the fence standing.
#[test]
fn a_writer_outside_a_box_is_kept_from_no_source() {
    let host = Host::new();
    let (a, b, docs) = (host.worktree("a"), host.worktree("b"), package(&host));
    let unconfined = |run: &str| Unknown::Unconfined {
        site: "work".into(),
        run: run.into(),
    };
    let standing = host.admit(&open(&b, ["run-b", "b1"])).unwrap();
    let held = host.admit(&writing(&a, ["run-a", "a1"], &[&a])).unwrap();
    let refused = Err(unconfined("run-b").into());
    assert_eq!(extend(&held, inputs(&host, &docs)), refused);
    freed(&standing);
    freed(&held);
    let _serving = serving(&host, &a, ["run-a", "a2"], &docs);
    assert_eq!(
        host.admit(&open(&b, ["run-b", "b2"])),
        mb3(unconfined("run-b"))
    );
}

/// The extension is bounded and refuses participation nothing vouches
/// for, as admission does: an unknown participant in the protected root,
/// a lock held past the deadline, a lease whose attempt no longer stands,
/// and a deadline spent before a writer is compared each refuse by MB3's
/// identity cause, sealing nothing.
#[test]
fn an_extension_refuses_what_admission_refuses() {
    let host = Host::new();
    let (a, docs) = (host.worktree("a"), package(&host));
    let serves = || inputs(&host, &docs);
    let held = host.admit(&writing(&a, ["run-a", "a1"], &[&a])).unwrap();
    let before = std::fs::read(host.reservation(&a, "run-a")).unwrap();
    let stray = host.root().join("stray");
    std::fs::write(&stray, "").unwrap();
    let unknown = Err(Unknown::Participant(stray.clone()).into());
    assert_eq!(extend(&held, serves()), unknown);
    std::fs::remove_file(&stray).unwrap();
    let lock = taken(&host.root().join(LOCK));
    let (within, commits) = (
        Instant::now() + Duration::from_millis(200),
        Cell::new(Commit::Synced),
    );
    let expired = Err(Unknown::Expired.into());
    assert_eq!(extended(&held, serves(), within, &commits), expired);
    drop(lock);
    assert_eq!(
        std::fs::read(host.reservation(&a, "run-a")).unwrap(),
        before
    );
    freed(&held);
    assert_eq!(extend(&held, serves()), Err(Unknown::Lost.into()));
    let writer = boxed("work", &[&a]);
    let spent = Budget::new(Instant::now());
    let compared = kept(
        [("run-a", &Writing::from(&writer))],
        &serves(),
        &host.root,
        &spent,
    );
    assert_eq!(compared, Err(Unknown::Expired));
}

/// L1: an extension whose seal commits unsynced folds that into what its
/// caller holds, though it returns no error.
#[test]
fn an_extension_committed_unsynced_says_so() {
    let host = Host::new();
    let (a, docs) = (host.worktree("a"), package(&host));
    let held = host.admit(&writing(&a, ["run-a", "a1"], &[&a])).unwrap();
    unsynced_next();
    assert_eq!(extend(&held, inputs(&host, &docs)), Ok(Commit::Unsynced));
}

/// Run three's first finding: a bare launch name, which a box locates on
/// its own search path, here `usr/bin` beside the HOME, as `usr/bin/server
/// -> srv/links/server -> opt/docs/bin/server`. It is reserved before any
/// survey by the route of the file it is located as: a writer confined to
/// `srv/links`, which holds neither the name's directory nor the program,
/// is refused by MB3's identity cause naming the link it could replace —
/// server first, writer first, and the two started together — and one
/// confined beside it is admitted. Swapped toward that link between the
/// survey and the extension after it, the launch is located again and
/// refuses that extension. An engine that died holding the lease, never
/// releasing it, still refuses the writer. The program is unchanged.
#[test]
fn a_bare_launch_name_is_reserved_by_the_route_it_is_located_by() {
    let host = Host::new();
    let docs = package(&host);
    let (links, other) = (host.dir.path().join("srv/links"), plain(&host, "srv/other"));
    let toward = links.join("server").display().to_string();
    let direct = docs.join("bin/server").display().to_string();
    linked(
        &host,
        &[("usr/bin/server", &toward), ("usr/bin/later", &direct)],
    );
    let bin = host.dir.path().join("usr/bin");
    let candidates = |name: &str| vec![bin.join(name)];
    let launched = |name: &str| {
        let argv = [name.to_string()];
        [
            fixed(&bootstrap(&host)),
            launches_with([&argv[..]], &candidates),
        ]
        .concat()
    };
    let (refused, before) = (links.join("server"), program(&docs));
    let refusing = |index| writer_first(&host, pair(&host, index), &links, launched("server"));
    assert_eq!(refusing(0), Err(exposing("work", "run-b", &refused).into()));
    let apart = writer_first(&host, pair(&host, 1), &other, launched("server"));
    assert_eq!(apart, Ok(Commit::Synced));
    raced(
        &host,
        pair(&host, 2),
        &links,
        &|| launched("server"),
        &refused,
    );
    let (a, b) = pair(&host, 3);
    let held = host.admit(&writing(&a, ["run-a", "a1"], &[&a])).unwrap();
    let standing = host
        .admit(&writing(&b, ["run-b", "b1"], &[&b, &links]))
        .unwrap();
    assert_eq!(
        extend(&held, launched("later")),
        Ok(Commit::Synced),
        "survey"
    );
    std::fs::remove_file(bin.join("later")).unwrap();
    std::os::unix::fs::symlink(&refused, bin.join("later")).unwrap();
    let swapped = Err(exposing("work", "run-b", &refused).into());
    assert_eq!(extend(&held, launched("later")), swapped, "swapped");
    freed(&standing);
    assert_eq!(extend(&held, launched("server")), Ok(Commit::Synced));
    let incoming = |attempt: &str| writing(&b, ["run-b", attempt], &[&b, &links]);
    assert_eq!(
        host.admit(&incoming("b2")),
        exposed("work", "run-b", &refused)
    );
    drop(held);
    assert_eq!(
        host.admit(&incoming("b3")),
        exposed("work", "run-b", &refused)
    );
    assert_eq!(program(&docs), before);
}

/// Run three's second finding: the box launcher, found as a link in
/// `srv/links` toward the program it runs, after a candidate in `early`
/// that does not stand, is reserved by its route before any survey, as is
/// that earlier candidate, which a writer could set ahead of it. A writer
/// confined to `srv/links` or to `early` is refused by MB3's identity cause
/// naming what it could replace or set, server first, and refuses the
/// extension, writer first; one confined beside them is admitted, and
/// extended beside. The launcher's bytes are unchanged.
#[test]
fn the_box_launchers_route_and_every_candidate_before_it_are_reserved() {
    let host = Host::new();
    let launcher = written(&host, "opt/bwrap/bwrap");
    let (early, links) = (plain(&host, "early"), plain(&host, "srv/links"));
    let other = plain(&host, "srv/other");
    std::os::unix::fs::symlink(&launcher, links.join("bwrap")).unwrap();
    let found = || vec![early.join("bwrap"), links.join("bwrap")];
    let serves = || fixed_with(&bootstrap(&host), found());
    let named = [(&links, links.join("bwrap")), (&early, early.join("bwrap"))];
    for (index, (writer, named)) in named.iter().enumerate() {
        let paths = (writer.as_path(), other.as_path());
        refused_in_either_order(&host, pair(&host, index), paths, named, &serves);
    }
    assert_eq!(std::fs::read(&launcher).unwrap(), b"brokkr");
}

/// Server first, then writer first, for a lease on the worktree `a`
/// extended to `serves()` before any survey: a writer on `b` confined to
/// `writer` is refused by MB3's identity cause naming `named`, at its own
/// admission before anything of it is sealed, and by the extension it
/// stands before; one confined to `other` is admitted beside the server,
/// and extended beside.
fn refused_in_either_order(
    host: &Host,
    (a, b): (PathBuf, PathBuf),
    (writer, other): (&Path, &Path),
    named: &Path,
    serves: &dyn Fn() -> Vec<Served>,
) {
    let held = host.admit(&writing(&a, ["run-a", "a1"], &[&a])).unwrap();
    assert_eq!(extend(&held, serves()), Ok(Commit::Synced));
    let incoming = writing(&b, ["run-b", "b1"], &[&b, writer]);
    let refused = exposed("work", "run-b", named);
    assert_eq!(host.admit(&incoming), refused, "{named:?}");
    let sealed = host.reservation(&b, "run-b").exists();
    assert!(!sealed, "nothing of it sealed");
    freed(
        &host
            .admit(&writing(&b, ["run-b", "b2"], &[&b, other]))
            .unwrap(),
    );
    freed(&held);
    let first = writer_first(host, (a.clone(), b.clone()), writer, serves());
    let refused = Err(exposing("work", "run-b", named).into());
    assert_eq!(first, refused, "{named:?}");
    let apart = writer_first(host, (a, b), other, serves());
    assert_eq!(apart, Ok(Commit::Synced), "{named:?}");
}

/// A lookup reserved from its candidates `candidates`, as an attempt's
/// lease is extended to it before any survey.
type Lookup<'l> = &'l dyn Fn(Vec<PathBuf>) -> Vec<Served>;

/// Run four's first finding, for each lookup — the box launcher's discovery
/// and a bare launch name's: an earlier candidate `early/<name> ->
/// srv/later/<name>`, dangling, lies ahead of the one found in `usr/bin`. A
/// writer confined to `srv/later`, which could make it live, is refused by
/// MB3's identity cause naming where it would lead: server first, before
/// anything of it is sealed, and writer first, by the lease's extension
/// before any survey spawns, so nothing ever launches through it; one
/// confined to `srv/other` is admitted beside it, and extended beside. The
/// candidate stays dangling.
#[test]
fn a_dangling_earlier_candidate_is_refused_before_anything_launches_through_it() {
    let host = Host::new();
    let (later, other) = (plain(&host, "srv/later"), plain(&host, "srv/other"));
    let boot = bootstrap(&host);
    let launcher = |candidates: Vec<PathBuf>| fixed_with(&boot, candidates);
    let launch = |candidates: Vec<PathBuf>| {
        let argv = ["server".to_string()];
        let candidates = |_: &str| candidates.clone();
        [fixed(&boot), launches_with([&argv[..]], &candidates)].concat()
    };
    let lookups: [(&str, Lookup<'_>); 2] = [("bwrap", &launcher), ("server", &launch)];
    for (index, (name, lookup)) in lookups.into_iter().enumerate() {
        let (found, dangling) = (written(&host, &format!("usr/bin/{name}")), later.join(name));
        let toward = dangling.display().to_string();
        linked(&host, &[(&format!("early/{name}"), &toward)]);
        let early = host.dir.path().join("early").join(name);
        let serves = || lookup(vec![early.clone(), found.clone()]);
        let paths = (later.as_path(), other.as_path());
        refused_in_either_order(&host, pair(&host, index), paths, &dangling, &serves);
        let absent = std::fs::symlink_metadata(&dangling).map(|_| ());
        let kind = absent.map_err(|error| error.kind());
        assert_eq!(kind, Err(std::io::ErrorKind::NotFound), "still dangling");
    }
}

/// Run three's link-identity finding: a link is reserved as its own entry,
/// not as what it names. A route through `users -> var/users` reserves that
/// link and the program it leads to, so a writer holding the directory the
/// link lies in, or the program's own, is refused by MB3's identity cause
/// naming it, and one confined beside the program below `var/users`, which
/// metadata followed through the link once took for the link, is kept apart.
#[test]
fn a_link_is_guarded_as_its_own_entry_not_as_what_it_names() {
    let host = Host::new();
    let d = host.dir.path();
    let program = written(&host, "var/users/u/srv/bin/server");
    let beside = plain(&host, "var/users/u/other");
    linked(&host, &[("users", "var/users")]);
    let budget = Budget::new(soon());
    let spelled = Served::Source(d.join("users/u/srv/bin/server"));
    let serves = routed(&[spelled], &budget).unwrap();
    let link = d.join("users");
    let expected = vec![Served::Link(link.clone()), Served::Source(program.clone())];
    assert_eq!(serves, expected);
    let compared = |root: &Path| {
        let writer = boxed("work", &[root]);
        kept(
            [("run-b", &Writing::from(&writer))],
            &serves,
            &host.root,
            &budget,
        )
        .err()
    };
    assert_eq!(
        compared(&beside),
        None,
        "apart from the link and the program"
    );
    assert_eq!(compared(d), Some(exposing("work", "run-b", &link)));
    let bin = program.parent().unwrap();
    assert_eq!(compared(bin), Some(exposing("work", "run-b", &program)));
}

/// The chief's H1: a served lease depends on the reservations that hold it,
/// whatever evidence is at stake. A zero-grant writer whose box reaches the
/// coordination root, at it or above it, is refused beside a standing
/// server, by MB3's identity cause naming the root, nothing of it sealed;
/// one kept from the root is still admitted.
#[test]
fn a_writer_that_reaches_the_coordination_root_is_refused_beside_a_standing_server() {
    let host = Host::new();
    let (a, b, docs) = (host.worktree("a"), host.worktree("b"), package(&host));
    let served = serving(&host, &a, ["run-a", "a1"], &docs);
    let records = host.root();
    for (attempt, reach) in [("b1", &records), ("b2", &host.home)] {
        let incoming = writing(&b, ["run-b", attempt], &[&b, reach]);
        assert_eq!(
            host.admit(&incoming),
            exposed("work", "run-b", &records),
            "{reach:?}"
        );
    }
    assert!(!host.reservation(&b, "run-b").exists());
    let kept_apart = writing(&b, ["run-b", "b3"], &[&b]);
    freed(&host.admit(&kept_apart).unwrap());
    freed(&served);
}

/// H1 in the other order: a server's lease is not extended beside a standing
/// record whose writer's box reaches the coordination root, by MB3's identity
/// cause naming the root, the reservation unchanged; beside one kept from the
/// root, it is. The record is planted, as no such writer is admitted now
/// ([`a_boxed_writer_that_reaches_the_coordination_root_is_refused_on_every_run`]):
/// one an earlier build sealed still stands in this order.
#[test]
fn a_server_is_refused_beside_a_standing_writer_that_reaches_the_coordination_root() {
    let host = Host::new();
    let (a, b, docs) = (host.worktree("a"), host.worktree("b"), package(&host));
    let records = host.root();
    let standing = host.admit(&writing(&b, ["run-b", "b1"], &[&b])).unwrap();
    let reaching = vec![boxed("work", &[&b, &records])];
    planted_writers(&host, &b, ["run-b", "b1"], reaching);
    let held = host.admit(&writing(&a, ["run-a", "a1"], &[&a])).unwrap();
    let before = std::fs::read(host.reservation(&a, "run-a")).unwrap();
    assert_eq!(
        extend(&held, inputs(&host, &docs)),
        Err(exposing("work", "run-b", &records).into())
    );
    assert_eq!(
        std::fs::read(host.reservation(&a, "run-a")).unwrap(),
        before
    );
    let apart = vec![boxed("work", &[&b])];
    planted_writers(&host, &b, ["run-b", "b1"], apart);
    assert_eq!(extend(&held, inputs(&host, &docs)), Ok(Commit::Synced));
    freed(&standing);
    freed(&held);
}

/// The operator's ruling of 2026-10-30 on the chief's H2: with nothing served
/// and nothing at stake, a boxed writer whose reach covers the coordination
/// root, at it, above it, below it or through a link toward it, is refused by
/// MB3's identity cause naming the root, nothing of it sealed; one apart is
/// admitted.
#[test]
fn a_boxed_writer_that_reaches_the_coordination_root_is_refused_on_every_run() {
    let host = Host::new();
    let work = host.worktree("work");
    let records = host.root();
    let toward = host.dir.path().join("toward");
    std::os::unix::fs::symlink(&records, &toward).unwrap();
    for (attempt, reach) in [
        ("w1", records.clone()),
        ("w2", host.home.clone()),
        ("w3", records.join("below")),
        ("w4", toward),
    ] {
        let incoming = writing(&work, ["run-w", attempt], &[&work, &reach]);
        assert_eq!(
            host.admit(&incoming),
            exposed("work", "run-w", &records),
            "{reach:?}"
        );
    }
    assert!(!host.reservation(&work, "run-w").exists());
    freed(
        &host
            .admit(&writing(&work, ["run-w", "w5"], &[&work]))
            .unwrap(),
    );
}

/// The same refusal as the mount table alone shows it: a writer whose root is
/// a bind of the coordination root's own directory, which neither its path nor
/// its identity leads into, on a run with nothing at stake. Beside a table
/// that does not prove where the root lies, a boxed writer is refused too.
#[cfg(target_os = "linux")]
#[test]
fn a_bind_of_the_coordination_root_is_refused_as_the_mount_table_shows_it() {
    let host = Host::new();
    let (work, alias) = (host.worktree("work"), host.worktree("alias"));
    let records = host.root();
    let incoming = writing(&work, ["run-w", "w1"], &[&work, &alias]);
    let bound = Shown::with(&[(7, &records, &alias)]);
    assert_eq!(host.admit(&incoming), exposed("work", "run-w", &records));
    drop(bound);
    let _torn = Shown::text("not a mount table\n".into());
    assert_eq!(
        host.admit(&incoming),
        mb3(Unknown::Unmounted(records.clone()))
    );
    assert!(!host.reservation(&work, "run-w").exists());
}

/// H2's route, end to end. A record whose writers were cleared in place, the
/// forgery the chief named, is taken as written, so a broker extends beside
/// it: no check of the record can close this, and nothing here tries. The
/// route is closed at the door instead: the boxed writer that would forge it
/// is never admitted, with nothing served and nothing at stake, so no
/// reservation of it stands to clear. A writer that reaches the root only
/// beside a served lease is refused the same way, the lease's bytes kept.
#[test]
fn a_forged_record_has_no_boxed_writer_to_forge_it() {
    let host = Host::new();
    let (a, b, docs) = (host.worktree("a"), host.worktree("b"), package(&host));
    let records = host.root();
    let forger = writing(&b, ["run-b", "b1"], &[&b, &records]);
    assert_eq!(host.admit(&forger), exposed("work", "run-b", &records));
    assert!(!host.reservation(&b, "run-b").exists());
    let planted = host.admit(&writing(&b, ["run-b", "b2"], &[&b])).unwrap();
    planted_writers(&host, &b, ["run-b", "b2"], Vec::new());
    let held = host.admit(&writing(&a, ["run-a", "a1"], &[&a])).unwrap();
    assert_eq!(extend(&held, inputs(&host, &docs)), Ok(Commit::Synced));
    let lease = std::fs::read(host.reservation(&a, "run-a")).unwrap();
    let late = writing(&b, ["run-b", "b3"], &[&b, &records]);
    assert_eq!(host.admit(&late), exposed("work", "run-b", &records));
    assert_eq!(std::fs::read(host.reservation(&a, "run-a")).unwrap(), lease);
    freed(&planted);
    freed(&held);
}

/// A HOME in `host` under a group-writable parent, which [`Root::of`] will not
/// open, as the 0775 case of
/// [`the_root_resolves_from_home_only_where_every_ancestor_protects`] stands
/// one: that parent, and the HOME.
fn unprotected(host: &Host) -> (PathBuf, PathBuf) {
    let group = host.dir.path().join("group");
    let home = group.join("home");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::set_permissions(&group, std::fs::Permissions::from_mode(0o775)).unwrap();
    std::fs::set_permissions(&home, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(Root::of(&home), None, "the walk is not protected");
    (group, home)
}

/// A boxed seat of `binds`, on a fresh engine: its directory, the engine, and
/// the file its driver touches if it runs.
fn seat_binding(binds: &[&Path]) -> (tempfile::TempDir, Engine, PathBuf) {
    let (dir, mut engine) = engine(single_body(Vec::new()));
    let ran = dir.path().join("ran");
    let body = driver(&format!("touch '{}'", ran.display()));
    engine.bundle.seats.get_mut("work").unwrap().body = single_body(body);
    set_site_hands(&mut engine.bundle, "work", hands(binds));
    engine.boundary = brokkr_core::realms::Boundary::Namespace;
    (dir, engine, ran)
}

/// Whether the work seat's attempt is admitted under `home`, its root opened
/// as production's resolution opens it.
fn admitted_under(engine: &mut Engine, home: &Path) -> bool {
    let seat = engine.bundle.seats["work"].clone();
    let (body, _) = seat.body.selected(None).unwrap();
    let root = || Root::of(home);
    let ids = ["effect", "attempt", "work"];
    let coordination: Coordination<'_> = (home.to_path_buf(), &root);
    let selection = Selection::new();
    let admitted = engine.admit_in(coordination, ids, body, &selection, soon());
    admitted.unwrap()
}

/// The controller's ruling on the chief's H3: where the walk to the
/// coordination root is not protected, a boxed writer whose bind covers that
/// root, at it or above it, is refused by MB3's identity cause naming the
/// root, journaled, holding no lease, and nothing of the root is made. A box
/// beside the root under the same HOME, and a writer Brokkr cannot box,
/// proceed holding no lease.
#[test]
fn a_boxed_writer_reaching_a_root_that_cannot_be_opened_is_refused() {
    let host = Host::new();
    let (_group, home) = unprotected(&host);
    let records = coordination(&home);
    let beside = home.join("work");
    std::fs::create_dir(&beside).unwrap();
    for reach in [&records, &home] {
        let (_dir, mut seat, _) = seat_binding(&[reach]);
        assert!(!admitted_under(&mut seat, &home), "{reach:?}");
        let (event, payload) = last(&seat);
        assert_eq!(event, EventType::EffectFailed, "{payload}");
        let refused = Unadmitted::from(exposing("work", &seat.run_id, &records));
        assert_eq!(payload["error"], refused.to_string());
        assert_eq!(seat.held, None);
        assert!(!home.join(".local").exists(), "nothing was made or sealed");
    }
    let (_dir, mut apart, _) = seat_binding(&[&beside]);
    assert!(admitted_under(&mut apart, &home));
    assert_eq!(apart.held, None);
    let (_dir, mut unboxed, _) = seat_binding(&[&home]);
    unboxed.boundary = brokkr_core::realms::Boundary::Open;
    assert!(admitted_under(&mut unboxed, &home));
    assert_eq!(unboxed.held, None);
    assert!(!home.join(".local").exists(), "nothing was made or sealed");
}

/// The same refusal where the writer's reach cannot be proved: a mount table
/// that does not prove where the unopened root lies. A writer Brokkr cannot
/// box is compared with nothing, so it does not refuse that writer.
#[test]
fn a_boxed_writers_reach_of_an_unopened_root_that_cannot_be_proved_is_refused() {
    let host = Host::new();
    let work = host.worktree("work");
    let records = coordination(&host.dir.path().join("never/home"));
    let boxed_in = writing(&work, ["run-w", "w1"], &[&work]);
    let budget = Budget::new(soon());
    let keep = |claim: &Claim, root: &Path| records_kept(claim, root, shown, &budget);
    assert_eq!(keep(&boxed_in, &records), Ok(()), "the table proves it");
    let _torn = Shown::text("not a mount table\n".into());
    let unproved = Unknown::Unmounted(records.clone());
    assert_eq!(keep(&boxed_in, &records), Err(unproved));
    assert_eq!(keep(&open(&work, ["run-w", "w2"]), &records), Ok(()));
}

/// U9b1a's final review, L1: where the host names no HOME, `Engine::admit`
/// has only an empty root to compare, and a boxed writer is refused by that
/// cause, journaled and holding no lease; the mount table is not blamed.
#[test]
fn a_boxed_writer_is_refused_by_its_own_cause_where_the_host_names_no_home() {
    let host = Host::new();
    let beside = host.worktree("beside");
    let (_dir, mut seat, _) = seat_binding(&[&beside]);
    let observed = crate::engine::broker::observation::Observed {
        host: Err(crate::engine::broker::Unprovisioned::Root),
        survey: &crate::engine::broker::surveyed,
        home: None,
        open: |_| None,
    };
    let body = seat.bundle.seats["work"].clone();
    let (body, _) = body.body.selected(None).unwrap();
    let timed = (Instant::now(), Duration::from_secs(30));
    let ids = ["effect", "attempt", "work"];
    let admitted = seat.admit(ids, body, &Selection::new(), timed, &observed);
    assert!(!admitted.unwrap());
    let (event, payload) = last(&seat);
    assert_eq!(event, EventType::EffectFailed, "{payload}");
    assert_eq!(
        payload["error"],
        "dispatch refused: MCP server box filesystem identity is not protected: this host names \
         no HOME, so no boxed writer is proved apart from its coordination root"
    );
    assert_eq!(seat.held, None);
}

/// H3's transition, each step's outcome exact. The walk not protected, the
/// root-reaching writer is refused, so no such writer is live. The walk then
/// repaired, as that writer could have done from inside its box: a later
/// attempt opens the root, is admitted, and extends a broker's lease beside
/// nothing unrecorded, and the same writer is now refused under the lock with
/// the lease's bytes kept. Skipped with its reason where no location protects
/// a HOME, as the repaired walk then opens nothing.
#[test]
fn a_root_reaching_writer_refused_unopened_leaves_a_later_broker_alone() {
    let Some(host) = Host::resolving() else {
        return;
    };
    let (group, home) = unprotected(&host);
    let records = coordination(&home);
    let (_dir, mut reaching, ran) = seat_binding(&[&home]);
    assert!(!admitted_under(&mut reaching, &home));
    let refused = Unadmitted::from(exposing("work", &reaching.run_id, &records));
    assert_eq!(last(&reaching).1["error"], refused.to_string());
    assert!(!ran.exists(), "no driver started");
    std::fs::set_permissions(&group, std::fs::Permissions::from_mode(0o700)).unwrap();
    let root = Arc::new(Root::of(&home).expect("the walk is repaired"));
    assert_eq!(root.path(), records);
    let a = host.worktree("a");
    let docs = package(&host);
    let broker = writing(&a, ["run-a", "a1"], &[&a]);
    let held = admit(&root, &broker, soon()).unwrap();
    assert_eq!(extend(&held, inputs(&host, &docs)), Ok(Commit::Synced));
    let lease = records.join(&broker.repo).join("run-a").join(RESERVATION);
    let bytes = std::fs::read(&lease).unwrap();
    let later = writing(&a, ["run-a", "a2"], &[&a, &home]);
    assert_eq!(
        admit(&root, &later, soon()),
        exposed("work", "run-a", &records)
    );
    assert_eq!(std::fs::read(&lease).unwrap(), bytes);
    freed(&held);
}
