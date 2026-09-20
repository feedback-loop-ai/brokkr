//! The differential matrix (design D10, proposal AO, third hold): every
//! commissioned program spelling crossed with every layout, the resolver
//! compared with a real `std::process::Command::new(name)` child under
//! IDENTICAL cwd and environment. Each candidate is a sentinel with its
//! own identity — a script printing a unique marker, or a native image
//! (this test binary, hard-linked in, answering `--list`) — so a cell
//! asserts which FILE the platform ran, never that something ran.
//!
//! Every cell has its own oracle, and the parent asserts that it was
//! invoked: the child prints one `matrix-oracle:` line per completed
//! native outcome, and the set of those identifiers must equal the
//! inventory the parent declares from the same names and layouts. A
//! cell that returned early, an oracle that was skipped, a sentinel that
//! exited unsuccessfully or printed nothing identifiable cannot become
//! a pass (run `09ec8d81`, R6).
//!
//! Every ordinary cell is equality: the resolver selects exactly the
//! canonical file the child executed, or refuses exactly where the child
//! got NotFound; a terminal native error (ENAMETOOLONG, ELOOP on glibc,
//! EACCES on a direct path) is a refusal by cause, and B is never
//! selected in its place. The obstruction cells are the one named
//! exception: native lookup walks past an `A/dsh` whose interpreter or
//! dynamic loader is missing and runs B, and the resolver refuses at A
//! naming the prerequisite, recorded separately as D10's exception and
//! never counted as an equality pass. A NUL-bearing name is refused
//! before any lookup where the child reports invalid input.
//!
//! The six component lengths are each their own layout — 255, 256, 300,
//! 4095, 4096 and 5000 ASCII `x` bytes ahead of a runnable B — because
//! one 5,000-byte cell proved a continuation glibc makes BEFORE
//! `execve` and said nothing about the `ENAMETOOLONG` glibc returns
//! AFTER it: 256, 300 and 4095 stop the native search with errno 36,
//! and the resolver that walked past them authorized an execution
//! native lookup rejects (run `09ec8d81`, R1). On glibc each of the six
//! is asserted by its own expected outcome beside the generic parity.
//!
//! Two invocation forms are run for every cell. The INHERITED form is
//! production's: the layout's `PATH` is staged in this test binary's own
//! environment by the parent, and the child calls `Command::new(name)`
//! with no environment change beside `resolve_executable`, which reads
//! the same environment — the `posix_spawnp` path Rust takes for an
//! unchanged `PATH`. The EXPLICIT form sets the child's `PATH` on the
//! `Command`, which Rust 1.88 (`unix.rs` 417–423) turns into `fork` and
//! `execvp`; on glibc both run `__execvpe_common` and are asserted
//! equal, and on other targets the explicit control is recorded beside
//! the inherited assertion rather than assumed equal, because Apple's
//! `execvP` and `posix_spawnp` size their buffers differently.
//!
//! The working directory is process-wide state, so each layout runs in
//! a CHILD of this test binary whose cwd is the fixture.
//!
//! Removal controls recorded in the delivery account: restoring blanket
//! `ENAMETOOLONG` continuation fails the 256/300/4095 cells' terminal
//! cause; removing the pre-execution buffer skip fails the 4096/5000
//! cells' B identity; restoring backslash-as-separator fails the
//! `C:\Tools\dsh.exe` cwd-only cell; restoring absent-PATH-as-empty-
//! entry fails the `dsh` absent-PATH cell; restoring unconditional
//! absent-PATH refusal fails the `sh` default-search control; restoring
//! unconditional native-image or one-level interpreter admission fails
//! the loader cells; omitting one oracle fails the inventory assertion.

use super::super::*;
use super::*;
use std::collections::BTreeSet;
use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Stdio};

/// The variable that makes this test binary a matrix child: the layout
/// index it runs, or `controls` for the cells outside the cross-product.
const CASE: &str = "BROKKR_COMPOSITE_NATIVE_MATRIX";

/// The loader path the patched ELF names, which must not exist.
const MISSING_LOADER: &str = "/no-such-ld-9f3";

/// The commissioned component lengths, each its own layout.
const LENGTHS: [usize; 6] = [255, 256, 300, 4095, 4096, 5000];

/// The cells outside the name × layout cross-product, each with its own
/// oracle: the overlong bare name, its explicit-path spelling, the
/// `NAME_MAX` boundary, the valid-length positive and the default-search
/// positive.
const CONTROLS: [&str; 5] = [
    "overlong-bare-name",
    "overlong-explicit-path",
    "name-max-boundary",
    "valid-length-name",
    "default-search-sh",
];

/// What a candidate is made of.
enum Body {
    /// A script printing its marker.
    Script,
    /// A mode-0644 script no child executes.
    NonExecutable,
    /// A script naming an interpreter that does not exist.
    MissingInterpreter,
    /// A script whose interpreter is the native image with no loader.
    InterpreterMissingLoader,
    /// This test binary, hard-linked: a working native image.
    Native,
    /// The native image whose `PT_INTERP` names a missing loader.
    Broken,
    /// A symlink to itself.
    SelfSymlink,
}

enum Expect {
    /// Resolver equals the native child: same file, or NotFound and a
    /// refusal, or a terminal error and a refusal by cause.
    Parity,
    /// D10's exception: native runs B, the resolver refuses naming A
    /// and its prerequisite.
    Obstruction,
}

struct Layout {
    name: &'static str,
    /// `(slot, body)` per candidate placed; `Target` and `A` coincide
    /// for a direct spelling, so a layout never places both.
    files: Vec<(Slot, Body)>,
    /// The child's `PATH`, spelled with the slot directories, or absent.
    path: Option<Vec<Slot>>,
    /// Whether `path`'s spelling carries an empty entry at the front,
    /// between, or at the end, or is the single empty entry.
    empty_at: Option<usize>,
    expect: Expect,
    /// Whether removing ONLY the first component is a control of its
    /// own, with a fresh oracle that must run the same B.
    removal: bool,
}

impl Layout {
    /// The component length this layout tests, when it is a length cell.
    fn length(&self) -> Option<usize> {
        self.path.as_ref().and_then(|slots| {
            slots.iter().find_map(|slot| match slot {
                Slot::Long(bytes) => Some(*bytes),
                _ => None,
            })
        })
    }
}

/// The layouts the matrix crosses on this target: the commissioned
/// cwd, `PATH`, absent, empty-entry, denial, native-image and loop
/// layouts; the six lengths; the regular-file and nonexistent
/// components; the ordered denial-then-miss and denial-then-terminal
/// controls; a skip followed by an empty entry; and, where the ELF
/// fixture can be built, the two missing-loader layouts.
fn layouts(loader_fixture: bool) -> Vec<Layout> {
    let parity = |name: &'static str, files: Vec<(Slot, Body)>, path: Vec<Slot>| Layout {
        name,
        files,
        path: Some(path),
        empty_at: None,
        expect: Expect::Parity,
        removal: false,
    };
    let mut layouts = vec![
        Layout {
            name: "cwd-only, PATH elsewhere",
            files: vec![(Slot::Target, Body::Script)],
            path: Some(vec![Slot::Other]),
            empty_at: None,
            expect: Expect::Parity,
            removal: false,
        },
        parity(
            "PATH directory plus competing cwd file",
            vec![(Slot::PathDir, Body::Script), (Slot::Target, Body::Script)],
            vec![Slot::PathDir],
        ),
        Layout {
            name: "PATH absent, cwd file",
            files: vec![(Slot::Target, Body::Script)],
            path: None,
            empty_at: None,
            expect: Expect::Parity,
            removal: false,
        },
        Layout {
            name: "present-empty PATH, cwd file",
            files: vec![(Slot::Target, Body::Script)],
            path: Some(vec![]),
            empty_at: Some(0),
            expect: Expect::Parity,
            removal: false,
        },
        Layout {
            name: "present-empty PATH, no candidate",
            files: vec![],
            path: Some(vec![]),
            empty_at: Some(0),
            expect: Expect::Parity,
            removal: false,
        },
        Layout {
            name: "leading empty entry",
            files: vec![(Slot::Target, Body::Script), (Slot::PathDir, Body::Script)],
            path: Some(vec![Slot::PathDir]),
            empty_at: Some(0),
            expect: Expect::Parity,
            removal: false,
        },
        Layout {
            name: "interior empty entry",
            files: vec![(Slot::Target, Body::Script), (Slot::Other, Body::Script)],
            path: Some(vec![Slot::PathDir, Slot::Other]),
            empty_at: Some(1),
            expect: Expect::Parity,
            removal: false,
        },
        Layout {
            name: "trailing empty entry",
            files: vec![(Slot::Target, Body::Script)],
            path: Some(vec![Slot::PathDir]),
            empty_at: Some(1),
            expect: Expect::Parity,
            removal: false,
        },
        Layout {
            name: "A:B, A has a missing interpreter",
            files: vec![(Slot::A, Body::MissingInterpreter), (Slot::B, Body::Script)],
            path: Some(vec![Slot::A, Slot::B]),
            empty_at: None,
            expect: Expect::Obstruction,
            removal: false,
        },
        // What the search does with a loop is each C library's own:
        // glibc stops with ELOOP, Apple continues to B. `Parity`
        // asserts the RUNNING platform's native control either way,
        // which is what the corrected AS1 cell requires and what a
        // fixed "Unix" outcome could not express (F3).
        parity(
            "A:B, A is a self-symlink (ELOOP)",
            vec![(Slot::A, Body::SelfSymlink), (Slot::B, Body::Script)],
            vec![Slot::A, Slot::B],
        ),
        parity(
            "A:B, A not executable",
            vec![(Slot::A, Body::NonExecutable), (Slot::B, Body::Script)],
            vec![Slot::A, Slot::B],
        ),
        // The sole candidate is one this process may not execute: the
        // child's search ends in EACCES, not ENOENT, and the resolver's
        // refusal names that denial (review 2026-09-20, R9).
        parity(
            "A alone, A not executable",
            vec![(Slot::A, Body::NonExecutable)],
            vec![Slot::A],
        ),
        parity(
            "A:B, A a working native image",
            vec![(Slot::A, Body::Native), (Slot::B, Body::Script)],
            vec![Slot::A, Slot::B],
        ),
        // Denial, then a miss: the remembered EACCES is what the child
        // reports, never a NotFound the search did not end in.
        parity(
            "A not executable, then a nonexistent component",
            vec![(Slot::A, Body::NonExecutable)],
            vec![Slot::A, Slot::Nowhere],
        ),
        // Denial, then a terminal error: the terminal cause wins over
        // the remembered denial, on glibc as ENAMETOOLONG.
        parity(
            "A not executable, then a 300-byte component",
            vec![(Slot::A, Body::NonExecutable)],
            vec![Slot::A, Slot::Long(300)],
        ),
        // The native construction skip is a skip, not a stop: iteration
        // goes on to the empty entry, and the cwd file runs.
        Layout {
            name: "4096-byte component, then an empty entry, cwd file",
            files: vec![(Slot::Target, Body::Script)],
            path: Some(vec![Slot::Long(4096)]),
            empty_at: Some(1),
            expect: Expect::Parity,
            removal: false,
        },
    ];
    // The causes glibc's switch walks past, each ahead of a runnable B
    // and each with a removed-component control.
    for (name, slot) in [
        ("regular-file component, then B", Slot::File),
        ("nonexistent component, then B", Slot::Nowhere),
    ] {
        layouts.push(Layout {
            name,
            files: vec![(Slot::B, Body::Script)],
            path: Some(vec![slot, Slot::B]),
            empty_at: None,
            expect: Expect::Parity,
            removal: true,
        });
    }
    // The six lengths, individually visible.
    for bytes in LENGTHS {
        layouts.push(Layout {
            name: match bytes {
                255 => "255-byte component, then B",
                256 => "256-byte component, then B",
                300 => "300-byte component, then B",
                4095 => "4095-byte component, then B",
                4096 => "4096-byte component, then B",
                _ => "5000-byte component, then B",
            },
            files: vec![(Slot::B, Body::Script)],
            path: Some(vec![Slot::Long(bytes), Slot::B]),
            empty_at: None,
            expect: Expect::Parity,
            removal: true,
        });
    }
    if loader_fixture {
        layouts.push(Layout {
            name: "A:B, A's interpreter has a missing loader",
            files: vec![
                (Slot::A, Body::InterpreterMissingLoader),
                (Slot::B, Body::Script),
            ],
            path: Some(vec![Slot::A, Slot::B]),
            empty_at: None,
            expect: Expect::Obstruction,
            removal: false,
        });
        layouts.push(Layout {
            name: "A:B, A a native image with a missing loader",
            files: vec![(Slot::A, Body::Broken), (Slot::B, Body::Script)],
            path: Some(vec![Slot::A, Slot::B]),
            empty_at: None,
            expect: Expect::Obstruction,
            removal: false,
        });
    }
    layouts
}

/// The commissioned spellings, in a fixed order both processes read.
fn names(root: &Path) -> Vec<String> {
    vec![
        "dsh".to_string(),
        "./dsh".to_string(),
        "../dsh".to_string(),
        root.join("abs").join("dsh").display().to_string(),
        "C:\\Tools\\dsh.exe".to_string(),
        "dsh.exe".to_string(),
        "my dsh".to_string(),
        "dsh\0x".to_string(),
    ]
}

/// Whether the ELF-loader layouts can be built on this target.
fn loader_fixture() -> bool {
    cfg!(target_os = "linux")
}

/// The directory one slot of one layout names, the same in the parent
/// that spells the child's `PATH` and in the child that places files.
fn dir_of(root: &Path, cwd: &Path, index: usize, slot: Slot) -> PathBuf {
    let layout = root.join(format!("layout-{index}"));
    match slot {
        Slot::PathDir => layout.join("path"),
        Slot::A => layout.join("a"),
        Slot::B => layout.join("b"),
        Slot::Other => layout.join("other"),
        // Never created, and never a candidate's home: the component
        // exists in the PATH spelling alone, relative to the cwd as the
        // commission spelled it.
        Slot::Long(bytes) => PathBuf::from("x".repeat(bytes)),
        Slot::File => layout.join("file-as-dir"),
        Slot::Nowhere => layout.join("nowhere"),
        Slot::Target => cwd.to_path_buf(),
    }
}

/// The child's `PATH` for one layout, or `None` for an absent one.
fn path_of(root: &Path, cwd: &Path, index: usize, layout: &Layout) -> Option<OsString> {
    layout.path.as_ref().map(|slots| {
        let mut entries: Vec<OsString> = slots
            .iter()
            .map(|slot| dir_of(root, cwd, index, *slot).into_os_string())
            .collect();
        if let Some(at) = layout.empty_at {
            entries.insert(at, OsString::new());
        }
        entries.join(std::ffi::OsStr::new(":"))
    })
}

/// One cell's identifier: name index, layout index and invocation form.
fn cell_id(name: usize, layout: usize, form: &str) -> String {
    format!("n{name}-l{layout}/{form}")
}

/// A completed native outcome: an identified sentinel that exited
/// successfully, or the spawn error with its kind and errno. Nothing
/// else is an outcome — unidentified output and an unsuccessful sentinel
/// exit are panics, never a pass.
enum Outcome {
    Ran(String),
    Failed(std::io::Error),
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Outcome::Ran(id) => write!(f, "ran {id}"),
            Outcome::Failed(error) => {
                write!(f, "failed: {error} (errno {:?})", error.raw_os_error())
            }
        }
    }
}

/// Invoke one oracle and RECORD it: the child prints the cell's
/// identifier so the parent can count it against the inventory.
fn oracle(command: &mut Command, cell: &str) -> Outcome {
    let outcome = match matrix_spawn(command) {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
            let id = match stdout.lines().find_map(|line| line.strip_prefix("MARK:")) {
                Some(id) => id.to_string(),
                None if stdout.contains("native_executable_resolution_matches_command_matrix") => {
                    "native".to_string()
                }
                None => panic!("{cell}: the child ran an unidentified file: {stdout}"),
            };
            assert!(
                output.status.success(),
                "{cell}: the sentinel {id} exited {:?}",
                output.status
            );
            Outcome::Ran(id)
        }
        Err(error) => Outcome::Failed(error),
    };
    // On its own line: libtest leaves `test … ... ` unterminated ahead
    // of a `--nocapture` test's first output.
    println!("\nmatrix-oracle: {cell}");
    outcome
}

/// How a cell was classified, for the tally.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Equal,
    NotFound,
    Terminal,
    Nul,
    Exception,
}

/// What one cell knows about its fixtures.
struct Cell<'a> {
    name: &'a str,
    direct: bool,
    layout: &'a Layout,
    /// `(marker, file)` per identified candidate placed.
    ids: &'a [(String, PathBuf)],
    obstructed: Option<&'a Path>,
    b: PathBuf,
}

/// Compare one completed native outcome with one resolution, by the
/// layout's expectation, and answer how the cell was classified. Every
/// arm asserts identity or the specific cause; none passes on a boolean.
fn compare(
    cell: &Cell<'_>,
    label: &str,
    native: &Outcome,
    resolved: &Result<PathBuf, CompositeError>,
) -> Kind {
    let describe = |what: &str| {
        format!(
            "{what}: {label}, name {:?}, layout {:?}, native {native:?}, resolver {resolved:?}",
            cell.name, cell.layout.name
        )
    };
    let refusal = |what: &str| match resolved {
        Ok(_) => panic!("{}", describe(what)),
        Err(error) => error.to_string(),
    };
    let placed = |id: &str| {
        cell.ids
            .iter()
            .find(|(known, _)| known == id)
            .map(|(_, at)| at.canonicalize().unwrap())
    };
    match (&cell.layout.expect, native) {
        (_, Outcome::Failed(error)) if error.kind() == std::io::ErrorKind::InvalidInput => {
            assert!(cell.name.contains('\0'), "{}", describe("invalid input"));
            assert_eq!(
                refusal("a NUL name was selected"),
                "the DSH layout is unreadable: 'dsh\\0x' carries a NUL",
                "{}",
                describe("a NUL is refused up front")
            );
            Kind::Nul
        }
        (Expect::Parity, Outcome::Ran(id)) => {
            let expected =
                placed(id).unwrap_or_else(|| panic!("{}", describe("an unplaced marker")));
            assert_eq!(
                resolved.as_ref().ok(),
                Some(&expected),
                "{}",
                describe("the resolver selects exactly the file the child ran")
            );
            Kind::Equal
        }
        (Expect::Parity, Outcome::Failed(error))
            if error.kind() == std::io::ErrorKind::NotFound =>
        {
            let reason = refusal("the resolver selected a file where the child found nothing");
            assert!(
                !cell
                    .ids
                    .iter()
                    .any(|(_, at)| reason.contains(&at.display().to_string())
                        && !reason.contains("is not on")),
                "{}",
                describe("the resolver refused where the child found nothing")
            );
            Kind::NotFound
        }
        (Expect::Parity, Outcome::Failed(error)) => {
            // ENAMETOOLONG on a long component, ELOOP on the symlink,
            // EACCES on a direct non-executable file or an exhausted
            // search: a terminal native error is a refusal by cause,
            // and B is never selected in its place.
            let reason = refusal("the resolver selected a file where the child stopped");
            let b = cell.b.display().to_string();
            assert!(
                !reason.contains(&b),
                "{}",
                describe("a terminal error authorizes no later candidate")
            );
            let errno = errno_of(error);
            if errno == Some(rustix::io::Errno::LOOP) {
                assert!(
                    reason.contains("a symlink loop stops the lookup"),
                    "{}",
                    describe("ELOOP is named")
                );
            }
            if errno == Some(rustix::io::Errno::NAMETOOLONG) {
                assert!(
                    reason.contains("File name too long")
                        && reason.contains("on which the platform's lookup stops"),
                    "{}",
                    describe("ENAMETOOLONG is named as the stop it is")
                );
            }
            if error.kind() == std::io::ErrorKind::PermissionDenied {
                assert!(
                    reason.contains("is not executable by this process"),
                    "{}",
                    describe("EACCES is named as the denial it is")
                );
            }
            Kind::Terminal
        }
        (Expect::Obstruction, native) => {
            // Native lookup walks past A to B (a bare name) or fails at
            // the spelled file (a direct one); the resolver refuses at A
            // by the prerequisite it could not establish, and never
            // selects B.
            let a = cell.obstructed.unwrap();
            match native {
                Outcome::Ran(id) => {
                    assert_eq!(
                        placed(id),
                        Some(cell.b.canonicalize().unwrap()),
                        "{}",
                        describe("the native child walked past A and ran B")
                    );
                }
                Outcome::Failed(error) => {
                    assert!(cell.direct, "{}", describe("only a direct spelling fails"));
                    assert_eq!(
                        error.kind(),
                        std::io::ErrorKind::NotFound,
                        "{}",
                        describe("the missing prerequisite is ENOENT to the child")
                    );
                }
            }
            // The refusal names the candidate as it was looked up: the
            // spelled path for a direct name, `A/<name>` for a searched
            // one.
            let named = match cell.direct {
                true => cell.name.to_string(),
                false => a.display().to_string(),
            };
            let reason = refusal("the resolver admitted the obstructed A");
            assert!(
                reason.starts_with(&format!("the DSH layout is unreadable: {named}: ")),
                "{}",
                describe("the refusal names A")
            );
            assert!(
                reason.contains("is missing:")
                    && (reason.contains("its #! interpreter")
                        || reason.contains("needs the ELF loader")),
                "{}",
                describe("the refusal names the prerequisite")
            );
            Kind::Exception
        }
    }
}

/// The glibc expectation of one length cell for a searched name, beside
/// the generic parity: the chief's six measured outcomes, each asserted
/// on its own so no cell can hide behind another.
fn assert_glibc_length(
    cell: &Cell<'_>,
    bytes: usize,
    native: &Outcome,
    resolved: &Result<PathBuf, CompositeError>,
) {
    let b = cell.b.canonicalize().unwrap();
    let describe = |what: &str| {
        format!(
            "{what}: {bytes}-byte component, name {:?}, native {native:?}, resolver {resolved:?}",
            cell.name
        )
    };
    match bytes {
        // Before `execve`: 4096 and 5000 reach glibc's buffer skip and
        // the search walks on to B; 255 is attempted, answers ENOENT,
        // and the switch walks on to B.
        255 | 4096 | 5000 => {
            let Outcome::Ran(id) = native else {
                panic!("{}", describe("native lookup runs B"));
            };
            assert_eq!(
                cell.ids
                    .iter()
                    .find(|(known, _)| known == id)
                    .map(|(_, at)| at.canonicalize().unwrap()),
                Some(b.clone()),
                "{}",
                describe("the native child ran B")
            );
            assert_eq!(
                resolved.as_ref().ok(),
                Some(&b),
                "{}",
                describe("the resolver selects B")
            );
        }
        // After `execve`: 256, 300 and 4095 are attempted and the kernel
        // answers ENAMETOOLONG, which is not in glibc's continue-set.
        _ => {
            let Outcome::Failed(error) = native else {
                panic!("{}", describe("native lookup stops with ENAMETOOLONG"));
            };
            assert_eq!(
                errno_of(error),
                Some(rustix::io::Errno::NAMETOOLONG),
                "{}",
                describe("the native error is ENAMETOOLONG")
            );
            let reason = match resolved {
                Ok(_) => panic!(
                    "{}",
                    describe("the resolver selected B where the child stopped")
                ),
                Err(error) => error.to_string(),
            };
            assert!(
                reason.contains("metadata answers File name too long (os error 36), on which the platform's lookup stops"),
                "{}",
                describe("the resolver refuses by the terminal cause")
            );
            assert!(
                !reason.contains(&b.display().to_string()),
                "{}",
                describe("B is never named")
            );
        }
    }
}

/// The parent: declares the inventory, stages each layout's `PATH` in a
/// child's environment, collects every oracle the children report and
/// asserts the two sets are one.
fn parent() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path();
    let cwd = root.join("cwd");
    fs::create_dir_all(&cwd).unwrap();
    fs::create_dir_all(root.join("abs")).unwrap();
    fs::create_dir_all(root.join("controls")).unwrap();
    let layouts = layouts(loader_fixture());
    let names = names(root);

    let mut declared: BTreeSet<String> = BTreeSet::new();
    for (l, layout) in layouts.iter().enumerate() {
        for (n, name) in names.iter().enumerate() {
            declared.insert(cell_id(n, l, "inherited"));
            declared.insert(cell_id(n, l, "explicit"));
            if layout.removal && !name.contains(['/', '\0']) {
                declared.insert(cell_id(n, l, "removed"));
            }
        }
    }
    for control in CONTROLS {
        declared.insert(format!("control:{control}"));
    }

    let mut executed: BTreeSet<String> = BTreeSet::new();
    let mut tally = [0usize; 5];
    let mut run = |case: &str, path: Option<OsString>| {
        let mut child = Command::new(std::env::current_exe().unwrap());
        child
            .args([
                "adapters::composite::tests::native_matrix::native_executable_resolution_matches_command_matrix",
                "--exact",
                "--nocapture",
                "--test-threads=1",
            ])
            .current_dir(&cwd)
            .env(CASE, case);
        match &path {
            Some(path) => child.env("PATH", path),
            None => child.env_remove("PATH"),
        };
        let output = spawn_retrying_etxtbsy(&mut child);
        let said = String::from_utf8_lossy(&output.stdout).into_owned()
            + &String::from_utf8_lossy(&output.stderr);
        assert!(
            said.contains("1 passed") || said.contains("1 failed"),
            "the child ran case {case} rather than filtering it away: {said}"
        );
        assert!(output.status.success(), "case {case}: {said}");
        for line in said.lines() {
            if let Some(id) = line.strip_prefix("matrix-oracle: ") {
                assert!(
                    executed.insert(id.to_string()),
                    "oracle {id} reported twice"
                );
            }
            if let Some(counts) = line.strip_prefix("matrix-tally: ") {
                for (slot, count) in counts.split(' ').enumerate() {
                    tally[slot] += count.parse::<usize>().unwrap();
                }
            }
        }
    };
    for (index, layout) in layouts.iter().enumerate() {
        run(&index.to_string(), path_of(root, &cwd, index, layout));
    }
    run("controls", Some(root.join("controls").into_os_string()));

    let missing: Vec<&String> = declared.difference(&executed).collect();
    let undeclared: Vec<&String> = executed.difference(&declared).collect();
    assert!(
        missing.is_empty() && undeclared.is_empty(),
        "every declared cell invoked its oracle and no other did; missing {missing:?}, undeclared {undeclared:?}"
    );
    let [equal, not_found, terminal, nul, exceptions] = tally;
    eprintln!(
        "matrix: {} names x {} layouts = {} cells in two invocation forms, plus removed-component \
         and named controls, {} oracles; {equal} equal selections, {not_found} NotFound parities, \
         {terminal} terminal-error parities, {nul} NUL refusals, {exceptions} D10 loader \
         exceptions recorded separately",
        names.len(),
        layouts.len(),
        names.len() * layouts.len(),
        executed.len()
    );
    assert!(equal > 0 && not_found > 0 && terminal > 0 && nul > 0);
    match loader_fixture() {
        true => assert!(exceptions > 0),
        false => eprintln!(
            "matrix: the two missing-loader layouts need an ELF fixture and are PENDING on this \
             target"
        ),
    }
}

/// One layout, in a child whose cwd and `PATH` the parent staged.
fn child_layout(index: usize) {
    let cwd = std::env::current_dir().unwrap();
    let root = cwd.parent().unwrap().to_path_buf();
    let layouts = layouts(loader_fixture());
    let layout = &layouts[index];
    let names = names(&root);
    let path = path_of(&root, &cwd, index, layout);
    assert_eq!(
        std::env::var_os("PATH"),
        path,
        "the parent staged this layout's PATH in this child's environment"
    );
    for slot in [Slot::PathDir, Slot::A, Slot::B, Slot::Other] {
        fs::create_dir_all(dir_of(&root, &cwd, index, slot)).unwrap();
    }
    fs::write(
        dir_of(&root, &cwd, index, Slot::File),
        b"a file, not a directory\n",
    )
    .unwrap();
    assert!(!dir_of(&root, &cwd, index, Slot::Nowhere).exists());

    // The native fixtures, built only where a layout places them: the
    // working copy of this binary, and the image whose loader is
    // missing — the working copy with only its `PT_INTERP` bytes
    // rewritten to a path that does not exist, so the native child's
    // ENOENT is the loader's and not a malformed header's.
    let needs_native = layout.files.iter().any(|(_, body)| {
        matches!(
            body,
            Body::Native | Body::Broken | Body::InterpreterMissingLoader
        )
    });
    let native_dir = root.join(format!("native-{index}"));
    let working = native_dir.join("working");
    let broken = native_dir.join("broken");
    if needs_native {
        fs::create_dir_all(&native_dir).unwrap();
        let exe = std::env::current_exe().unwrap();
        fs::copy(&exe, native_dir.join(".working.staging")).unwrap();
        fs::rename(native_dir.join(".working.staging"), &working).unwrap();
        if layout
            .files
            .iter()
            .any(|(_, body)| matches!(body, Body::Broken | Body::InterpreterMissingLoader))
        {
            assert!(!Path::new(MISSING_LOADER).exists());
            patch_elf_interpreter(&working, &broken, MISSING_LOADER);
            let error = Command::new(&broken).arg("--list").output().unwrap_err();
            assert_eq!(
                error.kind(),
                std::io::ErrorKind::NotFound,
                "the patched image fails natively for its missing loader: {error}"
            );
        }
    }
    let missing_interpreter = root.join("no-such-interpreter");

    let mut tally = [0usize; 5];
    let mut count = |kind: Kind| {
        tally[match kind {
            Kind::Equal => 0,
            Kind::NotFound => 1,
            Kind::Terminal => 2,
            Kind::Nul => 3,
            Kind::Exception => 4,
        }] += 1;
    };
    for (n, name) in names.iter().enumerate() {
        let name = name.as_str();
        let direct = name.contains('/');
        // The spelled file of a direct name, relative to the cwd the
        // child holds; a bare name's target is the cwd file of that name.
        let target: PathBuf = match name {
            "./dsh" => cwd.join("dsh"),
            "../dsh" => root.join("dsh"),
            _ if direct => PathBuf::from(name),
            _ => cwd.join(name),
        };
        // Where a candidate for this name goes in `slot`.
        let place_at = |slot: Slot| -> PathBuf {
            match (slot, direct) {
                (Slot::Target, _) | (Slot::A, true) => target.clone(),
                (slot, true) => dir_of(&root, &cwd, index, slot).join("dsh"),
                (slot, false) => dir_of(&root, &cwd, index, slot).join(name),
            }
        };
        let mut ids: Vec<(String, PathBuf)> = Vec::new();
        let mut placed: Vec<PathBuf> = Vec::new();
        let mut obstructed: Option<PathBuf> = None;
        if !name.contains('\0') {
            for (slot_index, (slot, body)) in layout.files.iter().enumerate() {
                let at = place_at(*slot);
                let id = format!("l{index}n{n}s{slot_index}");
                match body {
                    Body::Script => {
                        stage_executable(
                            at.parent().unwrap(),
                            at.file_name().unwrap().to_str().unwrap(),
                            format!("#!/bin/sh\nprintf 'MARK:{id}\\n'\n").as_bytes(),
                        );
                        ids.push((id, at.clone()));
                    }
                    Body::NonExecutable => {
                        fs::write(&at, b"#!/bin/sh\nprintf 'MARK:never\\n'\n").unwrap();
                        fs::set_permissions(&at, fs::Permissions::from_mode(0o644)).unwrap();
                    }
                    Body::MissingInterpreter => {
                        stage_executable(
                            at.parent().unwrap(),
                            at.file_name().unwrap().to_str().unwrap(),
                            format!("#!{}\n", missing_interpreter.display()).as_bytes(),
                        );
                        obstructed = Some(at.clone());
                    }
                    Body::InterpreterMissingLoader => {
                        stage_executable(
                            at.parent().unwrap(),
                            at.file_name().unwrap().to_str().unwrap(),
                            format!("#!{}\n", broken.display()).as_bytes(),
                        );
                        obstructed = Some(at.clone());
                    }
                    Body::Native => {
                        fs::hard_link(&working, &at).unwrap();
                        ids.push(("native".to_string(), at.clone()));
                    }
                    Body::Broken => {
                        fs::hard_link(&broken, &at).unwrap();
                        obstructed = Some(at.clone());
                    }
                    Body::SelfSymlink => {
                        std::os::unix::fs::symlink(at.file_name().unwrap(), &at).unwrap();
                    }
                }
                placed.push(at);
            }
        }
        let cell = Cell {
            name,
            direct,
            layout,
            ids: &ids,
            obstructed: obstructed.as_deref(),
            b: place_at(Slot::B),
        };
        let bare = !direct && !name.contains('\0');

        // The INHERITED form: production's invocation and production's
        // resolver, both reading this process's own environment.
        let id = cell_id(n, index, "inherited");
        let native = oracle(Command::new(name).arg("--list").stdin(Stdio::null()), &id);
        let resolved = resolve_executable(name);
        count(compare(&cell, &id, &native, &resolved));
        if let (Some(bytes), true, true, Library::Glibc) =
            (layout.length(), layout.removal, bare, LIBRARY)
        {
            assert_glibc_length(&cell, bytes, &native, &resolved);
        }

        // The EXPLICIT form: the same `PATH` set on the `Command`, and the
        // resolver handed the same value. Equal on glibc, where both
        // forms run one loop; recorded elsewhere.
        let id = cell_id(n, index, "explicit");
        let mut command = Command::new(name);
        command.arg("--list").stdin(Stdio::null());
        match &path {
            Some(path) => command.env("PATH", path),
            None => command.env_remove("PATH"),
        };
        let explicit = oracle(&mut command, &id);
        let resolved_explicit = resolve_executable_in(name, path.clone());
        match LIBRARY {
            Library::Glibc => {
                compare(&cell, &id, &explicit, &resolved_explicit);
                assert_eq!(
                    resolved_explicit.as_ref().ok(),
                    resolved.as_ref().ok(),
                    "{id}: both forms select alike on glibc"
                );
            }
            _ => eprintln!(
                "matrix: {id} recorded on {}: explicit-PATH native {explicit:?}, resolver \
                 {resolved_explicit:?}",
                std::env::consts::OS
            ),
        }

        // Removing ONLY the first component: a fresh oracle that must
        // run the same B, and a resolver that selects it.
        if layout.removal && bare {
            let id = cell_id(n, index, "removed");
            let without: OsString = dir_of(&root, &cwd, index, Slot::B).into_os_string();
            let mut command = Command::new(name);
            command
                .arg("--list")
                .stdin(Stdio::null())
                .env("PATH", &without);
            let native = oracle(&mut command, &id);
            let b = cell.b.canonicalize().unwrap();
            let Outcome::Ran(marker) = &native else {
                panic!("{id}: with the component removed the native child runs B: {native:?}");
            };
            assert_eq!(
                ids.iter()
                    .find(|(known, _)| known == marker)
                    .map(|(_, at)| at.canonicalize().unwrap()),
                Some(b.clone()),
                "{id}: the native child ran B"
            );
            assert_eq!(
                resolve_executable_in(name, Some(without)).unwrap(),
                b,
                "{id}: the resolver selects the same B"
            );
        }

        for at in placed {
            let _ = fs::remove_file(&at);
        }
    }
    println!(
        "\nmatrix-tally: {} {} {} {} {}",
        tally[0], tally[1], tally[2], tally[3], tally[4]
    );
}

/// The cells outside the cross-product, each with its own oracle, in a
/// child whose `PATH` is the parent's `controls` directory alone.
fn child_controls() {
    let cwd = std::env::current_dir().unwrap();
    let root = cwd.parent().unwrap().to_path_buf();
    let controls = root.join("controls");
    assert_eq!(
        std::env::var_os("PATH"),
        Some(controls.clone().into_os_string()),
        "the parent staged the controls directory as this child's PATH"
    );
    let record = |what: &str, native: &Outcome, resolved: &Result<PathBuf, CompositeError>| {
        eprintln!(
            "matrix control {what} on {}: native {native:?}, resolver {resolved:?}",
            std::env::consts::OS
        );
    };

    // An overlong bare program name: glibc refuses it with ENAMETOOLONG
    // before searching (`posix/execvpe.c` 92–106), and the resolver
    // refuses by that cause, never as an ordinary miss.
    let overlong = "x".repeat(300);
    let native = oracle(
        Command::new(&overlong).arg("--list").stdin(Stdio::null()),
        "control:overlong-bare-name",
    );
    let resolved = resolve_executable(&overlong);
    match LIBRARY {
        Library::Glibc => {
            let Outcome::Failed(error) = &native else {
                panic!("the native child ran a 300-byte name: {native:?}");
            };
            assert_eq!(
                errno_of(error),
                Some(rustix::io::Errno::NAMETOOLONG),
                "{error}"
            );
            assert_eq!(
                refused(resolved),
                format!(
                    "the DSH layout is unreadable: '{overlong}' is 300 bytes long, more than the \
                     255 bytes NAME_MAX allows a searched name, which the platform's lookup \
                     refuses with ENAMETOOLONG before searching"
                )
            );
        }
        _ => record("overlong-bare-name", &native, &resolved),
    }

    // The same name as an explicit path: `execve` itself answers
    // ENAMETOOLONG, and the resolver's one candidate stops by that cause.
    let spelled = controls.join(&overlong);
    let native = oracle(
        Command::new(&spelled).arg("--list").stdin(Stdio::null()),
        "control:overlong-explicit-path",
    );
    let resolved = resolve_executable(spelled.to_str().unwrap());
    match LIBRARY {
        Library::Glibc => {
            let Outcome::Failed(error) = &native else {
                panic!("the native child ran an overlong explicit path: {native:?}");
            };
            assert_eq!(
                errno_of(error),
                Some(rustix::io::Errno::NAMETOOLONG),
                "{error}"
            );
            assert_eq!(
                refused(resolved),
                format!(
                    "the DSH layout is unreadable: {}: metadata answers File name too long (os \
                     error 36), on which the platform's lookup stops",
                    spelled.display()
                )
            );
        }
        _ => record("overlong-explicit-path", &native, &resolved),
    }

    // The NAME_MAX boundary itself: 256 bytes is one more than glibc
    // searches for, and it is refused before any entry is read even
    // though the search directory holds nothing of that name.
    let boundary = "y".repeat(256);
    let native = oracle(
        Command::new(&boundary).arg("--list").stdin(Stdio::null()),
        "control:name-max-boundary",
    );
    let resolved = resolve_executable(&boundary);
    match LIBRARY {
        Library::Glibc => {
            let Outcome::Failed(error) = &native else {
                panic!("the native child ran a 256-byte name: {native:?}");
            };
            assert_eq!(
                errno_of(error),
                Some(rustix::io::Errno::NAMETOOLONG),
                "{error}"
            );
            assert!(
                refused(resolved).contains("is 256 bytes long, more than the 255 bytes NAME_MAX"),
                "the boundary is the library's own"
            );
        }
        _ => record("name-max-boundary", &native, &resolved),
    }

    // The valid-length positive: a 255-byte name that EXISTS on the
    // search runs natively and is selected, so length continuation and
    // length refusal are both measured against a name that runs.
    let valid = "z".repeat(255);
    let planted = stage_executable(
        &controls,
        &valid,
        b"#!/bin/sh\nprintf 'MARK:valid-255\\n'\n",
    );
    let native = oracle(
        Command::new(&valid).arg("--list").stdin(Stdio::null()),
        "control:valid-length-name",
    );
    let Outcome::Ran(marker) = &native else {
        panic!("the native child runs a 255-byte name on its search: {native:?}");
    };
    assert_eq!(marker, "valid-255");
    assert_eq!(
        resolve_executable(&valid).unwrap(),
        planted.canonicalize().unwrap(),
        "the resolver selects the 255-byte name the child ran"
    );

    // The default-search positive AO requires beside the all-negative
    // absent-PATH cells: `sh` runs with PATH removed, and the resolver
    // selects the very file it ran, with a same-name cwd decoy present.
    stage_executable(&cwd, "sh", b"#!/bin/sh\nprintf 'MARK:decoy\\n'\n");
    let output = matrix_spawn(
        Command::new("sh")
            .args(["-c", "readlink /proc/$$/exe 2>/dev/null || echo unknown"])
            .env_remove("PATH"),
    )
    .expect("a native child finds sh with no PATH");
    println!("\nmatrix-oracle: control:default-search-sh");
    let ran = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert!(output.status.success() && !ran.contains("decoy"), "{ran}");
    let selected = resolve_executable_in("sh", None).unwrap();
    assert_ne!(selected, cwd.join("sh").canonicalize().unwrap());
    if cfg!(target_os = "linux") {
        assert_eq!(
            selected,
            PathBuf::from(&ran),
            "the resolver's absent-PATH selection is the native default-search identity"
        );
    }
    println!("\nmatrix-tally: 0 0 0 0 0");
}

#[test]
fn native_executable_resolution_matches_command_matrix() {
    match std::env::var(CASE) {
        Ok(case) if case == "controls" => child_controls(),
        Ok(case) => child_layout(case.parse().unwrap()),
        Err(_) => parent(),
    }
}
