//! The differential matrix (design D10, proposal AO, third and fourth
//! holds): every commissioned program spelling crossed with every
//! layout, the resolver compared with a real `std::process::Command::new
//! (name)` child under IDENTICAL cwd and environment. Each candidate is a
//! sentinel with its own identity — a script printing a unique marker,
//! or a native image (this test binary, hard-linked in, answering
//! `--list`) — so a cell asserts which FILE the platform ran, never that
//! something ran.
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
//! selected in its place. Two kinds of cell are STRICTER than native, and
//! each is asserted as its own kind, never counted as equality:
//!
//! - the obstruction cells (D10's exception): native lookup walks past an
//!   `A/dsh` whose interpreter or dynamic loader is missing and runs B,
//!   and the resolver refuses at A naming the prerequisite;
//! - the working-directory cells (the fourth hold's reconciled rule):
//!   where the platform's own candidate sequence reaches the working
//!   directory — an explicit empty entry, or the implicit iteration
//!   glibc produces after skipping an oversized component — native runs
//!   whatever sits there, stops on it, or walks past it to B, and the
//!   resolver refuses by the named cwd reason, or by native's own
//!   terminal cause where native stops there (ELOOP on a cwd
//!   self-symlink). Never cwd, otherwise native.
//!
//! A NUL-bearing name is refused before any lookup where the child
//! reports invalid input.
//!
//! A layout DECLARES the expectation glibc's rule gives it, because
//! glibc is where every cell was measured, and `expect_on` translates
//! that declaration to the running library and this form's operation.
//! The translation exists because the declaration is not portable: the
//! oversized-component layouts reach the working directory only on
//! glibc, whose skip leaves the cursor on the colon, while Apple sizes
//! every candidate against a 1,024-byte buffer BEFORE building it, so
//! the same `PATH` stops `posix_spawnp` with `ENAMETOOLONG` having run
//! nothing and is merely skipped by `execvP`. Reading one library's
//! declaration as though it were the rule is what made PR #311's macOS
//! leg cost one cell per pass.
//!
//! Every failing case is COLLECTED and reported in one panic, by the
//! child for its cells and by the parent for its children. The matrix
//! is a cross-product of independent cells, and this suite cannot be
//! run on one of its hosts: three macOS passes at ~25 minutes each
//! answered twelve failures, then one, then two, each hiding the next
//! (2026-09-21). One pass now shows the whole remaining surface.
//!
//! The six component lengths are each their own layout — 255, 256, 300,
//! 4095, 4096 and 5000 ASCII `x` bytes ahead of a runnable B — because
//! one 5,000-byte cell proved a continuation glibc makes BEFORE `execve`
//! and said nothing about the `ENAMETOOLONG` glibc returns AFTER it: 256,
//! 300 and 4095 stop the native search with errno 36 (run `09ec8d81`,
//! R1). The two that are skipped, 4096 and 5000, are each crossed with
//! the working directory holding a runnable candidate, a self-symlink,
//! or nothing (run `efb3360b`, R1): the chief reproduced native running
//! `cwd/dsh` and native stopping ELOOP/40 where the resolver had
//! authorized B. So are `A::B`, `A:`, `:B` and `PATH=""`.
//!
//! Two invocation forms are run for every cell, and each is compared on
//! its own. The INHERITED form is production's: the layout's `PATH` is
//! staged in this test binary's own environment by the parent, and the
//! child calls `Command::new(name)` with no environment change beside
//! `resolve_executable`, which reads the same environment — the
//! `posix_spawnp` path Rust takes for an unchanged `PATH`. The EXPLICIT
//! form sets the child's `PATH` on the `Command`, which Rust 1.88
//! (`unix.rs` 417–423) turns into `fork` and `execvp`, compared with
//! `resolve_executable_in`, the resolver's `execvp` operation. On glibc
//! both run `__execvpe_common` and are additionally asserted equal; on
//! other targets each form's own comparison stands and no equality is
//! assumed, because Apple's `execvP` and `posix_spawnp` size their
//! buffers differently.
//!
//! The working directory is process-wide state, so each layout runs in
//! a CHILD of this test binary whose cwd is the fixture; a layout's
//! same-fixture removal control runs in a second child whose staged
//! `PATH` lacks only the removed component (or one slash), with the cwd
//! and every placed file unchanged.
//!
//! Removal controls recorded in the delivery account: restoring the
//! direct advance to B after an oversized skip fails the competing-cwd
//! and looping-cwd 4096/5000 cells; removing the cwd refusal fails every
//! explicit-empty and implicit-cwd cell; removing the pre-execution
//! buffer skip fails those cells' cwd reason with an invented
//! ENAMETOOLONG; restoring blanket `ENAMETOOLONG` continuation fails the
//! 256/300/4095 cells' terminal cause; normalizing the extra slash fails
//! the padded-A cells; restoring backslash-as-separator fails the
//! `C:\Tools\dsh.exe` cwd-only cell; restoring absent-PATH-as-empty-entry
//! fails the `dsh` absent-PATH cell; restoring unconditional absent-PATH
//! refusal fails the `sh` default-search control; restoring unconditional
//! native-image or one-level interpreter admission fails the loader
//! cells; omitting one oracle fails the inventory assertion. Giving glibc
//! Apple's metadata rule fails the sealed-directory-alone controls in
//! both forms, and suppressing glibc's remembered access denial fails
//! the non-executable-then-sealed-directory controls in both.

use super::super::*;
use super::*;
use std::collections::BTreeSet;
use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Stdio};

/// The variable that makes this test binary a matrix child: the layout
/// index it runs (with a `:removed` suffix for the same-fixture removal
/// control), or `controls` for the cells outside the cross-product.
const CASE: &str = "BROKKR_COMPOSITE_NATIVE_MATRIX";

/// The loader path the patched ELF names, which must not exist.
const MISSING_LOADER: &str = "/no-such-ld-9f3";

/// The commissioned component lengths, each its own layout.
const LENGTHS: [usize; 6] = [255, 256, 300, 4095, 4096, 5000];

/// The two lengths glibc skips before `execve`, crossed with the working
/// directory's three states; 4095 is attempted and stops, and is crossed
/// with the same three states to prove the stop comes first.
const CWD_LENGTHS: [usize; 3] = [4095, 4096, 5000];

/// The cells outside the name × layout cross-product, each with its own
/// oracle: the overlong bare name under an existing, a missing and a
/// file prefix, then missing-then-existing, at the working directory
/// and with no candidate at all; its explicit-path spelling; the
/// `NAME_MAX` boundary; the valid-length positive; the two ordered
/// exhaustion causes; the default-search positive; and a directory
/// without search permission as the only candidate and as the last one,
/// each in the explicit form and, in a child of its own, the inherited.
const CONTROLS: [&str; 17] = [
    "overlong-bare-name",
    "overlong-under-missing-prefix",
    "overlong-under-file-prefix",
    "overlong-missing-then-existing",
    "overlong-at-cwd",
    "overlong-no-candidate",
    "overlong-no-candidate-notdir",
    "overlong-explicit-path",
    "name-max-boundary",
    "valid-length-name",
    "exhaustion-missing-then-file",
    "exhaustion-file-then-missing",
    "default-search-sh",
    "sealed-directory-alone",
    "non-executable-then-sealed-directory",
    "sealed-directory-alone-inherited",
    "non-executable-then-sealed-directory-inherited",
];

/// The two sealed-directory `PATH`s, each run in both invocation forms.
const SEALED: [&str; 2] = [
    "sealed-directory-alone",
    "non-executable-then-sealed-directory",
];

/// The sealed-directory controls' fixtures: a `PATH` directory without
/// search permission holding a runnable `dsh` no unprivileged child
/// reaches, and a readable directory holding a regular `dsh` that is not
/// executable.
struct Sealed {
    sealed: PathBuf,
    hidden: PathBuf,
    readable: PathBuf,
    decoy: PathBuf,
}

impl Sealed {
    fn under(root: &Path) -> Sealed {
        let sealed = root.join("controls-sealed");
        let readable = root.join("controls-readable");
        Sealed {
            hidden: sealed.join("dsh"),
            decoy: readable.join("dsh"),
            sealed,
            readable,
        }
    }

    /// Staged by the parent, before any child that searches it runs.
    fn stage(&self) {
        fs::create_dir_all(&self.sealed).unwrap();
        stage_executable(&self.sealed, "dsh", b"#!/bin/sh\nprintf 'MARK:sealed\\n'\n");
        fs::create_dir_all(&self.readable).unwrap();
        fs::write(&self.decoy, b"#!/bin/sh\nprintf 'MARK:never\\n'\n").unwrap();
        fs::set_permissions(&self.decoy, fs::Permissions::from_mode(0o644)).unwrap();
        fs::set_permissions(&self.sealed, fs::Permissions::from_mode(0o000)).unwrap();
    }

    /// The `PATH` a sealed-directory control searches.
    fn path(&self, what: &str) -> OsString {
        match what {
            "sealed-directory-alone" => self.sealed.clone().into_os_string(),
            _ => OsString::from(format!(
                "{}:{}",
                self.readable.display(),
                self.sealed.display()
            )),
        }
    }

    /// Run one sealed-directory control's oracle and resolution, and
    /// assert native's exact errno and the resolver's whole refusal.
    ///
    /// A `PATH` directory without search permission, so its candidate's
    /// metadata cannot be read. glibc remembers that EACCES (`case
    /// EACCES: got_eacces = true;`). Apple's `default` walks past it
    /// unremembered (`if (stat(bp, &sb) != 0) break;`, `sys/posix_spawn.c`
    /// 186–187 and `gen/FreeBSD/exec.c` 282–283 at Libc-1752.120.2). As
    /// the only candidate, the search ends in EACCES on glibc and in
    /// ENOENT on Apple (posix_spawn.c 195–204, exec.c 293–306). As the
    /// last one, after a readable non-executable file, it ends in that
    /// file's denial on both. `posix_spawnp` and `execvP` read alike here,
    /// so both forms carry one expectation.
    fn control(
        &self,
        what: &str,
        id: &str,
        command: &mut Command,
        resolve: impl FnOnce() -> Result<PathBuf, CompositeError>,
    ) {
        let unreadable = fs::metadata(&self.hidden).unwrap_err();
        assert_eq!(
            errno_of(&unreadable),
            Some(rustix::io::Errno::ACCESS),
            "{id}: the sealed candidate's metadata is unreadable to this process"
        );
        let native = oracle(command.arg("--list").stdin(Stdio::null()), id);
        let resolved = resolve();
        let (errno, reason) = match (what, LIBRARY) {
            ("sealed-directory-alone", Library::Apple) => (
                rustix::io::Errno::NOENT,
                format!(
                    "the DSH layout is unreadable: 'dsh' is not on PATH (the search ended at {}: \
                     {unreadable})",
                    self.hidden.display()
                ),
            ),
            ("sealed-directory-alone", Library::Glibc) => (
                rustix::io::Errno::ACCESS,
                format!(
                    "the DSH layout is unreadable: 'dsh' is not executable by this process on \
                     PATH: {}: {unreadable}",
                    self.hidden.display()
                ),
            ),
            (_, Library::Apple | Library::Glibc) => (
                rustix::io::Errno::ACCESS,
                format!(
                    "the DSH layout is unreadable: 'dsh' is not executable by this process on \
                     PATH: {}: is not executable by this process",
                    self.decoy.display()
                ),
            ),
            _ => {
                return eprintln!(
                    "matrix control {id} on {}: native {native:?}, resolver {resolved:?}",
                    std::env::consts::OS
                )
            }
        };
        let Outcome::Failed(error) = &native else {
            panic!("{id}: the native child ran a dsh: {native:?}");
        };
        assert_eq!(errno_of(error), Some(errno), "{id}: {error}");
        assert_eq!(refused(resolved), reason, "{id}");
    }
}

/// Restores the sealed directory's search permission when dropped, so
/// the fixture tree can be removed whatever the controls answered.
struct Unseal(PathBuf);

impl Drop for Unseal {
    fn drop(&mut self) {
        let _ = fs::set_permissions(&self.0, fs::Permissions::from_mode(0o700));
    }
}

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

/// What the working directory holds under the searched name.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Cwd {
    /// A runnable script with its own marker.
    Runnable,
    /// A symlink to itself: ELOOP to whoever looks.
    Loop,
    /// Nothing.
    Absent,
}

impl Cwd {
    fn label(self) -> &'static str {
        match self {
            Cwd::Runnable => "cwd/dsh runnable",
            Cwd::Loop => "cwd/dsh a self-symlink",
            Cwd::Absent => "no cwd/dsh",
        }
    }

    fn file(self) -> Option<(Slot, Body)> {
        match self {
            Cwd::Runnable => Some((Slot::Target, Body::Script)),
            Cwd::Loop => Some((Slot::Target, Body::SelfSymlink)),
            Cwd::Absent => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Expect {
    /// Resolver equals the native child: same file, or NotFound and a
    /// refusal, or a terminal error and a refusal by cause.
    Parity,
    /// D10's exception: native runs B, the resolver refuses naming A
    /// and its prerequisite.
    Obstruction,
    /// The fourth hold's rule: the platform's sequence reaches the
    /// working directory; native runs it, stops on it or walks past it,
    /// and the resolver refuses there — by the named cwd reason, or by
    /// native's own terminal cause.
    WorkingDirectory,
    /// Apple's PRE-ATTEMPT bound: `posix_spawnp` sizes each candidate
    /// against its 1,024-byte buffer before building it (`lp + ln + 2 >
    /// sizeof(buf)`, `sys/posix_spawn.c` 131–134 at Libc-1752.120.2) and
    /// answers `ENAMETOOLONG`
    /// there, having handed `execve` nothing. Native therefore reports
    /// ENAMETOOLONG for a candidate that was never constructed, and the
    /// resolver names the BOUND rather than a stop the kernel never
    /// authored. Only the Apple arm produces this, and only under
    /// `posix_spawnp`: `execvP` warns and takes the next token.
    ConstructionStop,
}

/// The same-fixture control a layout carries.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Removal {
    None,
    /// Remove ONLY the first component and its delimiter: a fresh oracle
    /// must run the same B, and the resolver select it.
    Component,
    /// Remove exactly one of the padding slashes: the candidate fits,
    /// and a fresh oracle runs A when A holds it, B otherwise.
    OneSlash,
}

struct Layout {
    name: String,
    /// `(slot, body)` per candidate placed; `Target` and `A` coincide
    /// for a direct spelling, so a layout never places both.
    files: Vec<(Slot, Body)>,
    /// The child's `PATH`, spelled with the slot directories — an
    /// `Empty` slot is an empty entry — or absent.
    path: Option<Vec<Slot>>,
    expect: Expect,
    removal: Removal,
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

    /// The padded-A byte count, when this layout pads A.
    fn padded(&self) -> Option<usize> {
        self.path.as_ref().and_then(|slots| {
            slots.iter().find_map(|slot| match slot {
                Slot::PaddedA(bytes) => Some(*bytes),
                _ => None,
            })
        })
    }

    /// What the working directory holds in this layout.
    fn cwd(&self) -> Cwd {
        self.files
            .iter()
            .find_map(|(slot, body)| match (slot, body) {
                (Slot::Target, Body::Script) => Some(Cwd::Runnable),
                (Slot::Target, Body::SelfSymlink) => Some(Cwd::Loop),
                _ => None,
            })
            .unwrap_or(Cwd::Absent)
    }

    /// The `PATH` slots of this layout's removal control.
    fn removed(&self) -> Vec<Slot> {
        let slots = self.path.as_ref().expect("a removal layout spells a PATH");
        match self.removal {
            Removal::None => panic!("{}: no removal control", self.name),
            Removal::Component => slots[1..].to_vec(),
            Removal::OneSlash => slots
                .iter()
                .map(|slot| match slot {
                    Slot::PaddedA(bytes) => Slot::PaddedA(bytes - 1),
                    other => *other,
                })
                .collect(),
        }
    }
}

/// The layouts the matrix crosses on this target: the commissioned
/// cwd, `PATH`, absent, denial, native-image and loop layouts; the six
/// lengths, with 4095/4096/5000 each crossed with the working
/// directory's three states; `A::B`, `A:`, `:B` and `PATH=""` crossed
/// the same way, with their earlier-valid-A controls; the implicit and
/// explicit empty entries after a skip; the padded-A byte boundary; the
/// final oversized component; the regular-file and nonexistent
/// components; the ordered denial-then-miss and denial-then-terminal
/// controls; and, where the ELF fixture can be built, the two
/// missing-loader layouts.
fn layouts(loader_fixture: bool) -> Vec<Layout> {
    let parity = |name: &str, files: Vec<(Slot, Body)>, path: Vec<Slot>| Layout {
        name: name.to_string(),
        files,
        path: Some(path),
        expect: Expect::Parity,
        removal: Removal::None,
    };
    let mut layouts = vec![
        parity(
            "cwd-only, PATH elsewhere",
            vec![(Slot::Target, Body::Script)],
            vec![Slot::Other],
        ),
        parity(
            "PATH directory plus competing cwd file",
            vec![(Slot::PathDir, Body::Script), (Slot::Target, Body::Script)],
            vec![Slot::PathDir],
        ),
        Layout {
            name: "PATH absent, cwd file".to_string(),
            files: vec![(Slot::Target, Body::Script)],
            path: None,
            expect: Expect::Parity,
            removal: Removal::None,
        },
        Layout {
            name: "A:B, A has a missing interpreter".to_string(),
            files: vec![(Slot::A, Body::MissingInterpreter), (Slot::B, Body::Script)],
            path: Some(vec![Slot::A, Slot::B]),
            expect: Expect::Obstruction,
            removal: Removal::None,
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
        // A final oversized component: glibc breaks with nothing
        // constructed, so neither the cwd file nor anything else runs.
        parity(
            "5000-byte component alone, cwd/dsh runnable",
            vec![(Slot::Target, Body::Script)],
            vec![Slot::Long(5000)],
        ),
    ];
    // The working directory reached through the platform's OWN sequence,
    // in each of its three states.
    for cwd in [Cwd::Runnable, Cwd::Loop, Cwd::Absent] {
        let with_cwd = |mut files: Vec<(Slot, Body)>| {
            files.extend(cwd.file());
            files
        };
        // The native construction skip is a skip, not a stop, and its
        // next iteration is the working directory — implicitly on glibc
        // after 4096 and 5000; 4095 is attempted and stops first.
        for bytes in CWD_LENGTHS {
            layouts.push(Layout {
                name: format!("{bytes}-byte component, then B; {}", cwd.label()),
                files: with_cwd(vec![(Slot::B, Body::Script)]),
                path: Some(vec![Slot::Long(bytes), Slot::B]),
                expect: match bytes {
                    4095 => Expect::Parity,
                    _ => Expect::WorkingDirectory,
                },
                removal: Removal::Component,
            });
        }
        // The explicit empty entries, each at its position, with A an
        // existing directory holding no candidate.
        for (spelling, path, b) in [
            ("A::B", vec![Slot::Other, Slot::Empty, Slot::B], true),
            ("A:", vec![Slot::Other, Slot::Empty], false),
            (":B", vec![Slot::Empty, Slot::B], true),
            ("PATH=\"\"", vec![Slot::Empty], false),
        ] {
            let mut files = Vec::new();
            if b {
                files.push((Slot::B, Body::Script));
            }
            layouts.push(Layout {
                name: format!("{spelling}; {}", cwd.label()),
                files: with_cwd(files),
                path: Some(path),
                expect: Expect::WorkingDirectory,
                removal: Removal::None,
            });
        }
        // An explicit empty entry after a skipped component: the
        // implicit iteration comes first, and is the one refused.
        layouts.push(Layout {
            name: format!(
                "4096-byte component, then an empty entry, then B; {}",
                cwd.label()
            ),
            files: with_cwd(vec![(Slot::B, Body::Script)]),
            path: Some(vec![Slot::Long(4096), Slot::Empty, Slot::B]),
            expect: Expect::WorkingDirectory,
            removal: Removal::None,
        });
    }
    // Earlier success ends the search before the empty entry: a
    // runnable A is what both select, whatever the working directory
    // holds — and a blanket refusal of any PATH with an empty entry
    // would fail these.
    for cwd in [Cwd::Runnable, Cwd::Loop] {
        for (spelling, path) in [
            ("A::B", vec![Slot::A, Slot::Empty, Slot::B]),
            ("A:", vec![Slot::A, Slot::Empty]),
        ] {
            let mut files = vec![(Slot::A, Body::Script), (Slot::B, Body::Script)];
            files.extend(cwd.file());
            layouts.push(Layout {
                name: format!("{spelling}, A/dsh runnable; {}", cwd.label()),
                files,
                path: Some(path),
                expect: Expect::Parity,
                removal: Removal::None,
            });
        }
    }
    // The extra slash native construction appends to an entry that
    // already ends in one: A's spelling padded with `/` to 4,092 bytes
    // is a 4,096-byte `dsh` candidate, one over what the kernel takes,
    // whether or not A holds the file; one slash fewer fits (the
    // removal control), one more still does not.
    for (present, label) in [(true, "A/dsh present"), (false, "A/dsh absent")] {
        let mut files = vec![(Slot::B, Body::Script)];
        if present {
            files.insert(0, (Slot::A, Body::Script));
        }
        layouts.push(Layout {
            name: format!("A padded with slashes to 4092 bytes, then B; {label}"),
            files,
            path: Some(vec![Slot::PaddedA(4092), Slot::B]),
            expect: Expect::Parity,
            removal: Removal::OneSlash,
        });
    }
    layouts.push(parity(
        "A padded with slashes to 4093 bytes, then B; A/dsh present",
        vec![(Slot::A, Body::Script), (Slot::B, Body::Script)],
        vec![Slot::PaddedA(4093), Slot::B],
    ));
    // The causes glibc's switch walks past, each ahead of a runnable B
    // and each with a removed-component control.
    for (name, slot) in [
        ("regular-file component, then B", Slot::File),
        ("nonexistent component, then B", Slot::Nowhere),
    ] {
        layouts.push(Layout {
            name: name.to_string(),
            files: vec![(Slot::B, Body::Script)],
            path: Some(vec![slot, Slot::B]),
            expect: Expect::Parity,
            removal: Removal::Component,
        });
    }
    // The six lengths, individually visible, with nothing in the working
    // directory: the two skipped ones reach the empty cwd and are
    // refused there while native walks on to B.
    for bytes in LENGTHS {
        layouts.push(Layout {
            name: format!("{bytes}-byte component, then B"),
            files: vec![(Slot::B, Body::Script)],
            path: Some(vec![Slot::Long(bytes), Slot::B]),
            expect: match bytes {
                4096 | 5000 => Expect::WorkingDirectory,
                _ => Expect::Parity,
            },
            removal: Removal::Component,
        });
    }
    if loader_fixture {
        layouts.push(Layout {
            name: "A:B, A's interpreter has a missing loader".to_string(),
            files: vec![
                (Slot::A, Body::InterpreterMissingLoader),
                (Slot::B, Body::Script),
            ],
            path: Some(vec![Slot::A, Slot::B]),
            expect: Expect::Obstruction,
            removal: Removal::None,
        });
        layouts.push(Layout {
            name: "A:B, A a native image with a missing loader".to_string(),
            files: vec![(Slot::A, Body::Broken), (Slot::B, Body::Script)],
            path: Some(vec![Slot::A, Slot::B]),
            expect: Expect::Obstruction,
            removal: Removal::None,
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
        Slot::Empty => PathBuf::new(),
        // A's own spelling, padded with slashes to the byte count.
        Slot::PaddedA(bytes) => {
            let mut spelled = layout.join("a").into_os_string();
            let padding = bytes
                .checked_sub(spelled.len())
                .expect("the fixture root is shorter than the padded spelling");
            spelled.push("/".repeat(padding));
            PathBuf::from(spelled)
        }
    }
}

/// The child's `PATH` for the given slots.
fn spell(root: &Path, cwd: &Path, index: usize, slots: &[Slot]) -> OsString {
    let entries: Vec<OsString> = slots
        .iter()
        .map(|slot| dir_of(root, cwd, index, *slot).into_os_string())
        .collect();
    entries.join(std::ffi::OsStr::new(":"))
}

/// The child's `PATH` for one layout, or `None` for an absent one.
fn path_of(root: &Path, cwd: &Path, index: usize, layout: &Layout) -> Option<OsString> {
    layout
        .path
        .as_ref()
        .map(|slots| spell(root, cwd, index, slots))
}

/// One cell's identifier: name index, layout index and invocation form.
fn cell_id(name: usize, layout: usize, form: &str) -> String {
    format!("n{name}-l{layout}/{form}")
}

/// Whether Apple's walk can BUILD the candidate this slot and name make:
/// `lp + ln + 2 > sizeof(buf)` with `buf` PATH_MAX, an empty token
/// counted as the `.` Apple substitutes for it (`gen/FreeBSD/exec.c`
/// 194–197 and 215, `sys/posix_spawn.c` 110–113 and 131, both at
/// Libc-1752.120.2, `4e34d055`). The bound is per candidate and
/// applies to every token, so it is asked of every token rather than
/// assumed of the one the matrix pads.
fn apple_overflows(root: &Path, cwd: &Path, index: usize, slot: Slot, name: &str) -> bool {
    let lp = match slot {
        Slot::Empty => 1,
        _ => dir_of(root, cwd, index, slot).as_os_str().len(),
    };
    lp + name.len() + 2 > DARWIN_PATH_MAX
}

/// A layout's expectation UNDER THE RUNNING LIBRARY and this form's
/// operation.
///
/// A layout DECLARES the expectation glibc's rule gives it, because
/// glibc is where each cell was measured. That declaration is not
/// portable, and reading it as though it were is what made PR #311's
/// macOS leg reveal one cell per 25-minute pass: the oversized-component
/// layouts reach the working directory only on glibc, whose skip leaves
/// the cursor on the colon (`posix/execvpe.c` 118–124, 168). Apple has
/// no such iteration. Its bound is per candidate and 1,024 bytes, so an
/// oversized token STOPS `posix_spawnp` before any attempt and is SKIPPED
/// by `execvP`, which then walks on to whatever the next token is.
///
/// Only a layout's first component is ever oversized in this matrix —
/// the `Long` and `PaddedA` slots are always spelled first — and that is
/// asserted here rather than assumed, so a later layout cannot quietly
/// fall outside the correction.
fn expect_on(
    root: &Path,
    cwd: &Path,
    index: usize,
    layout: &Layout,
    name: &str,
    operation: Operation,
) -> Expect {
    let Some(slots) = layout.path.as_deref() else {
        return layout.expect;
    };
    let Some((first, rest)) = slots.split_first() else {
        return layout.expect;
    };
    if LIBRARY != Library::Apple {
        return layout.expect;
    }
    assert!(
        !rest
            .iter()
            .any(|slot| apple_overflows(root, cwd, index, *slot, name)),
        "{}: only a layout's FIRST component is oversized on Apple's arm",
        layout.name
    );
    if !apple_overflows(root, cwd, index, *first, name) {
        return layout.expect;
    }
    match operation {
        Operation::Spawn => Expect::ConstructionStop,
        // The skipped token is gone; an explicit empty entry behind it
        // is the next candidate, and is the working directory.
        Operation::Exec => match rest.contains(&Slot::Empty) {
            true => Expect::WorkingDirectory,
            false => Expect::Parity,
        },
    }
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
///
/// The identifier is printed as soon as the invocation COMPLETES, ahead
/// of identifying what ran. The two panics below are this cell's own
/// finding, collected by the caller like any other; printing after them
/// would also take the identifier out of the parent's inventory, so one
/// unidentified sentinel read as two findings — its panic and a missing
/// cell — and the second was a consequence of the first (review
/// 2026-09-21, R1).
fn oracle(command: &mut Command, cell: &str) -> Outcome {
    let spawned = matrix_spawn(command);
    // On its own line: libtest leaves `test … ... ` unterminated ahead
    // of a `--nocapture` test's first output.
    println!("\nmatrix-oracle: {cell}");
    match spawned {
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
    }
}

/// How a cell was classified, for the tally.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Equal,
    NotFound,
    Terminal,
    Nul,
    Exception,
    /// The fourth hold's policy refusal at the working directory,
    /// recorded apart from equality (AS1).
    WorkingDirectory,
}

const KINDS: usize = 6;

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

/// The reconciled rule's own words, which every working-directory
/// refusal carries.
const CWD_REASON: &str = "the platform's search would fall into the working directory";

/// Compare one completed native outcome with one resolution, by the
/// layout's expectation, and answer how the cell was classified. Every
/// arm asserts identity or the specific cause; none passes on a boolean.
fn compare(
    cell: &Cell<'_>,
    declared: Expect,
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
    // A direct spelling is the spelled file to native and resolver
    // alike, whatever the search would have reached: neither the cwd
    // rule nor Apple's per-candidate construction bound governs a
    // deliberate path — both are properties of the search's iteration.
    let expect = match (declared, cell.direct) {
        (Expect::WorkingDirectory | Expect::ConstructionStop, true) => Expect::Parity,
        (expect, _) => expect,
    };
    match (expect, native) {
        // A NUL name is refused by `Command` itself, before any spawn:
        // InvalidInput with no errno. (A search glibc breaks out of with
        // nothing attempted also surfaces as InvalidInput — the stale
        // errno 22 the child reports — and that one HAS an errno.)
        (_, Outcome::Failed(error))
            if error.kind() == std::io::ErrorKind::InvalidInput
                && error.raw_os_error().is_none() =>
        {
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
            assert!(
                !reason.contains(CWD_REASON),
                "{}",
                describe("a NotFound parity cell reached no working-directory candidate")
            );
            Kind::NotFound
        }
        (Expect::Parity, Outcome::Failed(error)) => {
            // ENAMETOOLONG on a long component, ELOOP on the symlink,
            // EACCES on a direct non-executable file or an exhausted
            // search, ENOTDIR at exhaustion after a file spelled as a
            // directory: a terminal native error is a refusal by cause,
            // and B is never selected in its place.
            let reason = refusal("the resolver selected a file where the child stopped");
            let b = cell.b.display().to_string();
            assert!(
                !reason.contains(&b),
                "{}",
                describe("a terminal error authorizes no later candidate")
            );
            let errno = errno_of(error);
            // WHO authored an errno, and whether it ended the lookup, is
            // per library and per position — so the wording owed is too,
            // and asserting glibc's on every host is what PR #311's
            // macOS leg kept failing on. Apple's switch CONTINUES past
            // both ELOOP and ENAMETOOLONG (`sys/posix_spawn.c` 146–150,
            // `gen/FreeBSD/exec.c` 232–235 at Libc-1752.120.2), so on
            // that arm a SEARCHED name can only end on one of them by
            // exhausting its entries, and the refusal is the exhaustion
            // it is; a DIRECT name, which no switch governs, names the
            // terminal cause on every arm alike.
            let terminal_here = cell.direct || LIBRARY != Library::Apple;
            if errno == Some(rustix::io::Errno::LOOP) {
                assert!(
                    match terminal_here {
                        true => reason.contains("a symlink loop stops the lookup"),
                        false =>
                            reason.contains("is not on")
                                && reason.contains("Too many levels of symbolic links"),
                    },
                    "{}",
                    describe("ELOOP is named as the stop or the exhaustion it is")
                );
            }
            if errno == Some(rustix::io::Errno::NAMETOOLONG) {
                assert!(
                    reason.contains("File name too long"),
                    "{}",
                    describe("ENAMETOOLONG is named")
                );
                assert!(
                    match terminal_here {
                        true => reason.contains("on which the platform's lookup stops"),
                        false => reason.contains("is not on"),
                    },
                    "{}",
                    describe("ENAMETOOLONG is named as the stop or the exhaustion it is")
                );
            }
            if errno == Some(rustix::io::Errno::NOTDIR) {
                assert!(
                    reason.contains("(os error 20)"),
                    "{}",
                    describe("ENOTDIR is the retained final cause")
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
        (Expect::ConstructionStop, native) => {
            // Apple's `posix_spawnp` answered ENAMETOOLONG for a
            // candidate it never built, so nothing ran — not B, and not
            // the working directory the same `PATH` reaches on glibc —
            // and the resolver names the BOUND, not a measurement.
            let reason = refusal("the resolver selected a file where construction stopped");
            let Outcome::Failed(error) = native else {
                panic!("{}", describe("the construction bound attempts nothing"));
            };
            assert_eq!(
                errno_of(error),
                Some(rustix::io::Errno::NAMETOOLONG),
                "{}",
                describe("the construction bound answers ENAMETOOLONG")
            );
            assert!(
                reason.contains(&format!(
                    "the platform's lookup stops before attempting a candidate longer than the \
                     {DARWIN_PATH_MAX} bytes it builds one in (ENAMETOOLONG)"
                )),
                "{}",
                describe("the resolver names the pre-attempt bound")
            );
            assert!(
                !reason.contains(&cell.b.display().to_string()) && !reason.contains(CWD_REASON),
                "{}",
                describe("a construction stop reaches neither B nor the working directory")
            );
            Kind::Terminal
        }
        (Expect::WorkingDirectory, native) => {
            // The platform's sequence reached the working directory.
            // Native ran what sat there (the cwd marker), walked past an
            // empty cwd to B or to NotFound, or stopped on the loop; the
            // resolver refused by the named reason in every case but
            // the stop, whose cause it preserves — and it never names B.
            let reason = refusal("the resolver selected a file where the search reached cwd");
            assert!(
                !reason.contains(&cell.b.display().to_string()),
                "{}",
                describe("a working-directory candidate authorizes no later candidate")
            );
            match native {
                Outcome::Ran(id) => {
                    let ran =
                        placed(id).unwrap_or_else(|| panic!("{}", describe("an unplaced marker")));
                    let cwd_file = std::env::current_dir()
                        .unwrap()
                        .canonicalize()
                        .unwrap()
                        .join(cell.name);
                    let b = cell.b.canonicalize().ok();
                    assert!(
                        ran == cwd_file || Some(ran.clone()) == b,
                        "{}",
                        describe("native ran the cwd candidate or walked past an empty cwd to B")
                    );
                    assert!(
                        reason.contains(CWD_REASON),
                        "{}",
                        describe("the cwd reason")
                    );
                    Kind::WorkingDirectory
                }
                Outcome::Failed(error) if errno_of(error) == Some(rustix::io::Errno::LOOP) => {
                    assert_eq!(
                        cell.layout.cwd(),
                        Cwd::Loop,
                        "{}",
                        describe("only the looping cwd stops native")
                    );
                    assert!(
                        reason.contains("a symlink loop stops the lookup"),
                        "{}",
                        describe("native's own terminal cause is preserved at cwd")
                    );
                    Kind::Terminal
                }
                Outcome::Failed(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    assert!(
                        reason.contains(CWD_REASON),
                        "{}",
                        describe("the cwd reason")
                    );
                    Kind::WorkingDirectory
                }
                Outcome::Failed(_) => panic!("{}", describe("an unexpected native outcome")),
            }
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
/// the generic comparison: the chief's measured outcomes, each asserted
/// on its own so no cell can hide behind another — the six lengths, and
/// the two skipped ones crossed with the working directory's states
/// (run `efb3360b`, R1).
fn assert_glibc_length(
    cell: &Cell<'_>,
    bytes: usize,
    native: &Outcome,
    resolved: &Result<PathBuf, CompositeError>,
) {
    let b = cell.b.canonicalize().unwrap();
    let cwd = cell.layout.cwd();
    let describe = |what: &str| {
        format!(
            "{what}: {bytes}-byte component, {}, name {:?}, native {native:?}, resolver {resolved:?}",
            cwd.label(),
            cell.name
        )
    };
    let ran_file = |id: &str| {
        cell.ids
            .iter()
            .find(|(known, _)| known == id)
            .map(|(_, at)| at.canonicalize().unwrap())
    };
    let cwd_file = std::env::current_dir()
        .unwrap()
        .canonicalize()
        .unwrap()
        .join(cell.name);
    let reason = |what: &str| match resolved {
        Ok(_) => panic!("{}", describe(what)),
        Err(error) => error.to_string(),
    };
    match (bytes, cwd) {
        // 255 is attempted, answers ENOENT, and the switch walks on to
        // B; nothing in cwd is consulted.
        (255, _) => {
            let Outcome::Ran(id) = native else {
                panic!("{}", describe("native lookup runs B"));
            };
            assert_eq!(
                ran_file(id),
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
        // 256, 300 and 4095 are attempted and the kernel answers
        // ENAMETOOLONG, which is not in glibc's continue-set: the search
        // stops before any cwd candidate, whatever cwd holds.
        (256 | 300 | 4095, _) => {
            let Outcome::Failed(error) = native else {
                panic!("{}", describe("native lookup stops with ENAMETOOLONG"));
            };
            assert_eq!(
                errno_of(error),
                Some(rustix::io::Errno::NAMETOOLONG),
                "{}",
                describe("the native error is ENAMETOOLONG")
            );
            let reason = reason("the resolver selected a file where the child stopped");
            assert!(
                reason.contains("metadata answers File name too long (os error 36), on which the platform's lookup stops"),
                "{}",
                describe("the resolver refuses by the terminal cause")
            );
            assert!(
                !reason.contains(&b.display().to_string()) && !reason.contains(CWD_REASON),
                "{}",
                describe("neither B nor cwd is reached")
            );
        }
        // 4096 and 5000 are skipped before `execve` and the next
        // iteration is the working directory: native runs the cwd file.
        (_, Cwd::Runnable) => {
            let Outcome::Ran(id) = native else {
                panic!("{}", describe("native lookup runs the cwd candidate"));
            };
            assert_eq!(
                ran_file(id),
                Some(cwd_file),
                "{}",
                describe("the native child ran cwd/dsh, not B")
            );
            let reason = reason("the resolver selected a file at the cwd iteration");
            assert!(
                reason.contains(&format!(
                    "{CWD_REASON}: glibc skips the {bytes}-byte component and its next iteration is the empty entry"
                )),
                "{}",
                describe("the resolver refuses by the named cwd reason")
            );
        }
        // …stops on the cwd self-symlink with ELOOP…
        (_, Cwd::Loop) => {
            let Outcome::Failed(error) = native else {
                panic!(
                    "{}",
                    describe("native lookup stops with ELOOP at the cwd loop")
                );
            };
            assert_eq!(
                errno_of(error),
                Some(rustix::io::Errno::LOOP),
                "{}",
                describe("the native error is ELOOP")
            );
            let reason = reason("the resolver selected a file where the child stopped");
            assert!(
                reason.contains("a symlink loop stops the lookup")
                    && reason.contains("(os error 40)"),
                "{}",
                describe("the resolver preserves native's ELOOP at cwd")
            );
            assert!(
                !reason.contains(&b.display().to_string()),
                "{}",
                describe("B is never named")
            );
        }
        // …or walks past an empty cwd to B, which the resolver still
        // does not follow.
        (_, Cwd::Absent) => {
            let Outcome::Ran(id) = native else {
                panic!(
                    "{}",
                    describe("native lookup walks past the empty cwd to B")
                );
            };
            assert_eq!(
                ran_file(id),
                Some(b.clone()),
                "{}",
                describe("the native child ran B")
            );
            let reason = reason("the resolver selected B past the cwd iteration");
            assert!(
                reason.contains(&format!(
                    "{CWD_REASON}: glibc skips the {bytes}-byte component and its next iteration is the empty entry"
                )),
                "{}",
                describe("the resolver refuses at cwd even with no cwd candidate")
            );
            assert!(
                !reason.contains(&b.display().to_string()),
                "{}",
                describe("B is never named")
            );
        }
    }
}

/// The glibc expectation of the padded-A cells for `dsh`: the 4,092-byte
/// spelling plus native's own `/` and the name is 4,096 bytes, which the
/// kernel refuses; the one-slash removal fits and selects A when A holds
/// the file, B otherwise.
fn assert_glibc_padded(
    cell: &Cell<'_>,
    bytes: usize,
    removed: bool,
    native: &Outcome,
    resolved: &Result<PathBuf, CompositeError>,
) {
    let describe = |what: &str| {
        format!(
            "{what}: A padded to {bytes} bytes{}, layout {:?}, native {native:?}, resolver {resolved:?}",
            if removed { " minus one slash" } else { "" },
            cell.layout.name
        )
    };
    let a_present = cell.layout.files.iter().any(|(slot, _)| *slot == Slot::A);
    match (bytes, removed) {
        (4092, true) => {
            let Outcome::Ran(id) = native else {
                panic!("{}", describe("the candidate fits and native runs it"));
            };
            let expected = match a_present {
                true => cell
                    .ids
                    .iter()
                    .find(|(_, at)| {
                        at.starts_with(at.parent().unwrap()) && at.parent().unwrap().ends_with("a")
                    })
                    .map(|(_, at)| at.canonicalize().unwrap())
                    .expect("A's marker is placed"),
                false => cell.b.canonicalize().unwrap(),
            };
            let ran = cell
                .ids
                .iter()
                .find(|(known, _)| known == id)
                .map(|(_, at)| at.canonicalize().unwrap());
            assert_eq!(
                ran,
                Some(expected.clone()),
                "{}",
                describe("native ran A when present, else B")
            );
            assert_eq!(
                resolved.as_ref().ok(),
                Some(&expected),
                "{}",
                describe("the resolver selects the same file through the padded spelling")
            );
        }
        _ => {
            let Outcome::Failed(error) = native else {
                panic!(
                    "{}",
                    describe("native construction is one byte over PATH_MAX")
                );
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
                    describe("the resolver selected a file where the child stopped")
                ),
                Err(error) => error.to_string(),
            };
            assert!(
                reason.contains("metadata answers File name too long (os error 36), on which the platform's lookup stops"),
                "{}",
                describe("the resolver keeps native's extra slash and its terminal cause")
            );
        }
    }
}

/// The parent: declares the inventory, stages each layout's `PATH` in a
/// child's environment (and its removal control's in a second child),
/// collects every oracle the children report and asserts the two sets
/// are one.
fn parent() {
    // The child reconstructs this root from its own `current_dir`, which
    // the kernel answers canonically, and then asserts that the `PATH`
    // the parent staged is the one it rebuilt. Where the temporary
    // directory is reached through a symlink — macOS's `/var` to
    // `/private/var` — the two spellings differ and every layout's first
    // assertion fails, so the root is resolved ONCE here and both sides
    // build on the same bytes.
    let root = FixtureRoot::new();
    let root = root.path();
    let cwd = root.join("cwd");
    fs::create_dir_all(&cwd).unwrap();
    fs::create_dir_all(root.join("abs")).unwrap();
    fs::create_dir_all(root.join("controls")).unwrap();
    // Declared after the root, so it is dropped first: the sealed
    // directory's permission is restored before the tree is removed,
    // including when an assertion below ends the parent.
    let sealed = Sealed::under(root);
    let _unseal = Unseal(sealed.sealed.clone());
    sealed.stage();
    let layouts = layouts(loader_fixture());
    let names = names(root);

    let mut declared: BTreeSet<String> = BTreeSet::new();
    for (l, layout) in layouts.iter().enumerate() {
        for (n, name) in names.iter().enumerate() {
            declared.insert(cell_id(n, l, "inherited"));
            declared.insert(cell_id(n, l, "explicit"));
            if layout.removal != Removal::None && !name.contains(['/', '\0']) {
                declared.insert(cell_id(n, l, "removed-inherited"));
                declared.insert(cell_id(n, l, "removed-explicit"));
            }
        }
    }
    for control in CONTROLS {
        declared.insert(format!("control:{control}"));
    }

    let mut executed: BTreeSet<String> = BTreeSet::new();
    let mut tally = [0usize; KINDS];
    // A failing child is RECORDED, never the end of the parent: the
    // layouts are independent of one another, and stopping at the first
    // makes every look at a host this seat cannot run — macOS — reveal
    // one layout and hide the rest (PR #311, three passes, 2026-09-21).
    let mut failures: Vec<String> = Vec::new();
    let mut run = |case: &str, path: Option<OsString>| {
        // The INVOCATION is collected too: a child this parent cannot
        // even spawn is one case's finding, and the layouts after it
        // are still run and still reported (review 2026-09-21, R1).
        let said = collecting(&mut failures, &format!("case {case}"), || {
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
            (output.status.success(), said)
        });
        let Some((ran, said)) = said else {
            return;
        };
        if !(said.contains("1 passed") || said.contains("1 failed")) {
            failures.push(format!(
                "case {case}: the child filtered the test away rather than running it: {said}"
            ));
        } else if !ran {
            failures.push(format!("case {case}: {said}"));
        }
        // The report this child wrote is VALIDATED as it is read, and a
        // report the parent cannot read is recorded beside the cells
        // rather than ending the run at the line it appeared on: a
        // duplicated identifier or an unreadable tally is a finding of
        // its own, and the layouts after it are still worth a look.
        for line in said.lines() {
            if let Some(id) = line.strip_prefix("matrix-oracle: ") {
                if !executed.insert(id.to_string()) {
                    failures.push(format!("case {case}: oracle {id} reported twice"));
                }
            }
            if let Some(counts) = line.strip_prefix("matrix-tally: ") {
                for (slot, count) in counts.split(' ').enumerate() {
                    match (tally.get_mut(slot), count.parse::<usize>()) {
                        (Some(total), Ok(count)) => *total += count,
                        _ => failures.push(format!(
                            "case {case}: a tally line this parent cannot read: {line}"
                        )),
                    }
                }
            }
        }
    };
    for (index, layout) in layouts.iter().enumerate() {
        run(&index.to_string(), path_of(root, &cwd, index, layout));
        if layout.removal != Removal::None {
            run(
                &format!("{index}:removed"),
                Some(spell(root, &cwd, index, &layout.removed())),
            );
        }
    }
    run("controls", Some(root.join("controls").into_os_string()));
    // The sealed-directory controls' INHERITED form, each in a child
    // whose own environment carries the `PATH` it searches.
    for what in SEALED {
        run(&format!("sealed:{what}"), Some(sealed.path(what)));
    }

    // Every failing case, in one panic, ahead of the inventory: a child
    // that failed also stopped reporting oracles, and its missing
    // identifiers are a consequence of the failure rather than a second
    // finding.
    report("the matrix", failures);
    let missing: Vec<&String> = declared.difference(&executed).collect();
    let undeclared: Vec<&String> = executed.difference(&declared).collect();
    assert!(
        missing.is_empty() && undeclared.is_empty(),
        "every declared cell invoked its oracle and no other did; missing {missing:?}, undeclared {undeclared:?}"
    );
    let [equal, not_found, terminal, nul, exceptions, working_directory] = tally;
    eprintln!(
        "matrix: {} names x {} layouts = {} cells in two invocation forms, plus removed-component \
         and named controls, {} oracles; {equal} equal selections, {not_found} NotFound parities, \
         {terminal} terminal-error parities, {nul} NUL refusals, {exceptions} D10 loader \
         exceptions and {working_directory} working-directory refusals recorded separately",
        names.len(),
        layouts.len(),
        names.len() * layouts.len(),
        executed.len()
    );
    assert!(equal > 0 && not_found > 0 && terminal > 0 && nul > 0 && working_directory > 0);
    match loader_fixture() {
        true => assert!(exceptions > 0),
        false => eprintln!(
            "matrix: the two missing-loader layouts need an ELF fixture and are PENDING on this \
             target"
        ),
    }
}

/// One layout, in a child whose cwd and `PATH` the parent staged —
/// either the layout's own `PATH`, or its removal control's.
fn child_layout(index: usize, removed: bool) {
    let cwd = std::env::current_dir().unwrap();
    let root = cwd.parent().unwrap().to_path_buf();
    let layouts = layouts(loader_fixture());
    let layout = &layouts[index];
    let names = names(&root);
    let path = match removed {
        true => Some(spell(&root, &cwd, index, &layout.removed())),
        false => path_of(&root, &cwd, index, layout),
    };
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

    let mut failures: Vec<String> = Vec::new();
    let mut tally = [0usize; KINDS];
    let mut count = |kind: Kind| {
        tally[match kind {
            Kind::Equal => 0,
            Kind::NotFound => 1,
            Kind::Terminal => 2,
            Kind::Nul => 3,
            Kind::Exception => 4,
            Kind::WorkingDirectory => 5,
        }] += 1;
    };
    let form = |base: &str| match removed {
        true => format!("removed-{base}"),
        false => base.to_string(),
    };
    // A removal control compares as a parity cell of the same fixtures:
    // the platform's search, with the offending component gone, and
    // the resolver, agreeing on the exact file. It carries its own
    // `PATH`, because a removal that leaves the component oversized on
    // Apple's arm — one slash off a 4,092-byte padded A is still far
    // over 1,024 — is still a construction stop there, and the
    // expectation has to be read from the spelling that actually ran.
    let removed_layout = Layout {
        name: format!("{} [removed]", layout.name),
        files: Vec::new(),
        path: match removed {
            true => Some(layout.removed()),
            false => None,
        },
        expect: Expect::Parity,
        removal: Removal::None,
    };
    for (n, name) in names.iter().enumerate() {
        let name = name.as_str();
        let direct = name.contains('/');
        let bare = !direct && !name.contains('\0');
        // A removal control is a bare-name cell: a direct spelling
        // searches nothing to remove from.
        if removed && !bare {
            continue;
        }
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
        let places_a = layout.files.iter().any(|(slot, _)| *slot == Slot::A);
        // The fixtures are staged under collection too, and what was
        // placed is recorded as it is placed, so a staging failure is
        // this cell's finding, the cells after it still run, and
        // whatever reached the filesystem is still removed below.
        let staged = collecting(&mut failures, &format!("n{n}-l{index} fixtures"), || {
            if name.contains('\0') {
                return;
            }
            for (slot_index, (slot, body)) in layout.files.iter().enumerate() {
                // `Target` and `A` coincide for a direct spelling: a
                // layout placing both keeps A there, and the cwd body is
                // a bare-name concern.
                if direct && places_a && *slot == Slot::Target {
                    continue;
                }
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
        });
        let cell = Cell {
            name,
            direct,
            layout,
            ids: &ids,
            obstructed: obstructed.as_deref(),
            b: place_at(Slot::B),
        };
        let glibc_bare = bare && LIBRARY == Library::Glibc;

        // A search that attempts nothing leaves the child's errno as this
        // thread had it: plant ENOENT so the final-oversize layout can
        // assert exactly that.
        let plant = |_: &str| {
            let planted = fs::metadata(root.join("nowhere-errno-plant")).unwrap_err();
            assert_eq!(errno_of(&planted), Some(rustix::io::Errno::NOENT));
        };
        // Each form's whole OPERATION under collection — the planted
        // errno, the oracle and the resolution together — so an oracle
        // that spawns nothing, runs an unidentified file or exits
        // unsuccessfully is this form's one finding and neither the end
        // of this cell's other form nor of the names after it (review
        // 2026-09-21, R1). A form with no outcome has nothing to
        // compare, and its comparison is skipped rather than invented.
        //
        // The INHERITED form: production's invocation and production's
        // resolver, both reading this process's own environment.
        let id = cell_id(n, index, &form("inherited"));
        let inherited = staged.and_then(|()| {
            collecting(&mut failures, &id, || {
                plant(&id);
                let native = oracle(Command::new(name).arg("--list").stdin(Stdio::null()), &id);
                (native, resolve_executable(name))
            })
        });
        // The EXPLICIT form: the same `PATH` set on the `Command`, and the
        // resolver handed the same value under its `execvp` operation.
        let id_explicit = cell_id(n, index, &form("explicit"));
        let explicit = staged.and_then(|()| {
            collecting(&mut failures, &id_explicit, || {
                let mut command = Command::new(name);
                command.arg("--list").stdin(Stdio::null());
                match &path {
                    Some(path) => command.env("PATH", path),
                    None => command.env_remove("PATH"),
                };
                plant(&id_explicit);
                let native = oracle(&mut command, &id_explicit);
                (native, resolve_executable_in(name, path.clone()))
            })
        });

        for (label, operation, invoked) in [
            (&id, Operation::Spawn, &inherited),
            (&id_explicit, Operation::Exec, &explicit),
        ] {
            let Some((native, resolved)) = invoked else {
                continue;
            };
            let kind = collecting(&mut failures, label, || match removed {
                // A same-fixture removal control is a parity cell: a
                // fresh oracle runs the same B (or A through the fitting
                // padded spelling), and the resolver selects it.
                true => {
                    let removed_cell = Cell {
                        layout: &removed_layout,
                        ..cell_view(&cell)
                    };
                    let expect = expect_on(&root, &cwd, index, &removed_layout, name, operation);
                    let kind = compare(&removed_cell, expect, label, native, resolved);
                    if layout.removal == Removal::Component {
                        let Outcome::Ran(marker) = native else {
                            panic!("{label}: with the component removed the native child runs B: {native:?}");
                        };
                        let b = cell.b.canonicalize().unwrap();
                        assert_eq!(
                            ids.iter()
                                .find(|(known, _)| known == marker)
                                .map(|(_, at)| at.canonicalize().unwrap()),
                            Some(b.clone()),
                            "{label}: the native child ran B"
                        );
                        assert_eq!(
                            resolved.as_ref().ok(),
                            Some(&b),
                            "{label}: the resolver selects the same B"
                        );
                    }
                    if let (Some(bytes), true, Removal::OneSlash) =
                        (layout.padded(), glibc_bare && name == "dsh", layout.removal)
                    {
                        assert_glibc_padded(&cell, bytes, true, native, resolved);
                    }
                    kind
                }
                false => {
                    let expect = expect_on(&root, &cwd, index, layout, name, operation);
                    let kind = compare(&cell, expect, label, native, resolved);
                    if let (Some(bytes), true) = (layout.length(), glibc_bare) {
                        if layout
                            .path
                            .as_ref()
                            .is_some_and(|slots| slots.len() == 2 && slots[1] == Slot::B)
                        {
                            assert_glibc_length(&cell, bytes, native, resolved);
                        }
                    }
                    if let (Some(bytes), true) = (layout.padded(), glibc_bare && name == "dsh") {
                        assert_glibc_padded(&cell, bytes, false, native, resolved);
                    }
                    // The final oversized component: glibc constructs
                    // nothing, native runs nothing, and the resolver
                    // says so — the cwd file is never reached.
                    if glibc_bare && layout.path.as_deref() == Some(&[Slot::Long(5000)]) {
                        // Nothing was attempted, so native's errno is
                        // whatever this thread had set before the fork
                        // (the planted ENOENT below), never the cwd
                        // marker and never a measured ENAMETOOLONG.
                        let Outcome::Failed(error) = native else {
                            panic!(
                                "{label}: a final oversized component attempts nothing: {native:?}"
                            );
                        };
                        assert_eq!(
                            errno_of(error),
                            Some(rustix::io::Errno::NOENT),
                            "{label}: nothing was constructed, so the planted errno stands: {error}"
                        );
                        let reason = match resolved {
                            Ok(selected) => panic!("{label}: selected {}", selected.display()),
                            Err(error) => error.to_string(),
                        };
                        assert!(
                            reason.contains("the search attempted no candidate"),
                            "{label}: {reason}"
                        );
                    }
                    kind
                }
            });
            if let Some(kind) = kind {
                count(kind);
            }
        }
        // Both forms' resolutions, where both forms have one: a form
        // whose invocation failed has recorded that failure already, and
        // comparing it with the other would report the same thing twice.
        if let (Some((_, resolved)), Some((_, resolved_explicit))) = (&inherited, &explicit) {
            collecting(&mut failures, &id, || {
                if LIBRARY == Library::Glibc {
                    assert_eq!(
                        resolved_explicit.as_ref().ok(),
                        resolved.as_ref().ok(),
                        "{id}: both forms select alike on glibc"
                    );
                }
            });
        }

        for at in placed {
            let _ = fs::remove_file(&at);
        }
    }
    println!(
        "\nmatrix-tally: {}",
        tally
            .iter()
            .map(|count| count.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    );
    report(
        &format!(
            "layout {index}{}",
            match removed {
                true => " [removed]",
                false => "",
            }
        ),
        failures,
    );
}

/// Run one cell's comparison and RECORD its failure rather than ending
/// the child at it.
///
/// The matrix is a cross-product, and a panic in the first cell is an
/// answer about that cell alone. On the one host this suite cannot run
/// locally that made each look cost a 25-minute CI pass and reveal one
/// cell: PR #311's macOS leg answered twelve failures, then one, then
/// two, each time a different cell behind the last (2026-09-21). A cell
/// is independent of its siblings — its fixtures are its own and are
/// removed after it — so collecting every failure and reporting them
/// together costs nothing and makes one pass show the whole surface.
fn collecting<T>(failures: &mut Vec<String>, label: &str, cell: impl FnOnce() -> T) -> Option<T> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(cell)) {
        Ok(answer) => Some(answer),
        Err(payload) => {
            let said = match payload.downcast_ref::<String>() {
                Some(said) => said.clone(),
                None => match payload.downcast_ref::<&str>() {
                    Some(said) => (*said).to_string(),
                    None => "a panic carrying no message".to_string(),
                },
            };
            failures.push(format!("{label}: {said}"));
            None
        }
    }
}

/// Every failure this child collected, in ONE panic, so a single pass on
/// a host this seat cannot reach reports its whole remaining surface.
fn report(what: &str, failures: Vec<String>) {
    assert!(
        failures.is_empty(),
        "{what}: {} of this child's cells failed:\n\n{}\n",
        failures.len(),
        failures.join("\n\n")
    );
}

/// A shallow copy of a cell's fixture view, for the removal control's
/// parity comparison under a parity layout of the same fixtures.
fn cell_view<'a>(cell: &Cell<'a>) -> Cell<'a> {
    Cell {
        name: cell.name,
        direct: cell.direct,
        layout: cell.layout,
        ids: cell.ids,
        obstructed: cell.obstructed,
        b: cell.b.clone(),
    }
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
    let mut failures: Vec<String> = Vec::new();
    let record = |what: &str, native: &Outcome, resolved: &Result<PathBuf, CompositeError>| {
        eprintln!(
            "matrix control {what} on {}: native {native:?}, resolver {resolved:?}",
            std::env::consts::OS
        );
    };
    // Each control's whole operation — its oracle and its resolution as
    // well as its comparison — sits inside its own `collecting`, for the
    // reason the cross-product cells do: a control whose oracle spawns
    // nothing or runs an unidentified file is one finding, and the
    // twelve controls after it are still worth the same pass (review
    // 2026-09-21, R1).
    let nowhere = root.join("controls-nowhere");
    assert!(!nowhere.exists());
    let file = root.join("controls-file");
    fs::write(&file, b"a file, not a directory\n").unwrap();
    // ENAMETOOLONG's NUMBER is the host's — 36 under Linux, 63 under
    // Darwin — so the expectation renders it from the same constant the
    // kernel answers with rather than spelling Linux's integer into a
    // control that also runs on macOS.
    let terminal = |candidate: &Path| {
        format!(
            "the DSH layout is unreadable: {}: metadata answers {}, on which the platform's \
             lookup stops",
            candidate.display(),
            std::io::Error::from_raw_os_error(rustix::io::Errno::NAMETOOLONG.raw_os_error())
        )
    };
    let exhausted = |name: &str, candidate: &Path| {
        format!(
            "the DSH layout is unreadable: '{name}' is not on PATH (the search ended at {}: {})",
            candidate.display(),
            fs::metadata(candidate).unwrap_err()
        )
    };

    // An overlong bare program name is NOT refused before the search
    // (fourth hold, R4): it meets the kernel under each entry, and the
    // entry decides. Under the existing controls directory the kernel
    // answers ENAMETOOLONG, on which glibc stops.
    let overlong = "x".repeat(300);
    collecting(&mut failures, "control:overlong-bare-name", || {
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
                assert_eq!(refused(resolved), terminal(&controls.join(&overlong)));
            }
            _ => record("overlong-bare-name", &native, &resolved),
        }
    });

    // Under a missing directory the name is never measured: ENOENT,
    // walked past, and the search is exhausted as NotFound.
    collecting(
        &mut failures,
        "control:overlong-under-missing-prefix",
        || {
            let native = oracle(
                Command::new(&overlong)
                    .arg("--list")
                    .stdin(Stdio::null())
                    .env("PATH", &nowhere),
                "control:overlong-under-missing-prefix",
            );
            let resolved = resolve_executable_in(&overlong, Some(nowhere.clone().into_os_string()));
            match LIBRARY {
                Library::Glibc => {
                    let Outcome::Failed(error) = &native else {
                        panic!(
                        "the native child ran a 300-byte name under a missing prefix: {native:?}"
                    );
                    };
                    assert_eq!(errno_of(error), Some(rustix::io::Errno::NOENT), "{error}");
                    assert_eq!(
                        refused(resolved),
                        exhausted(&overlong, &nowhere.join(&overlong))
                    );
                }
                _ => record("overlong-under-missing-prefix", &native, &resolved),
            }
        },
    );

    // Under a file spelled as a directory: ENOTDIR, walked past, and the
    // exhaustion keeps THAT cause.
    collecting(&mut failures, "control:overlong-under-file-prefix", || {
        let native = oracle(
            Command::new(&overlong)
                .arg("--list")
                .stdin(Stdio::null())
                .env("PATH", &file),
            "control:overlong-under-file-prefix",
        );
        let resolved = resolve_executable_in(&overlong, Some(file.clone().into_os_string()));
        match LIBRARY {
            Library::Glibc => {
                let Outcome::Failed(error) = &native else {
                    panic!("the native child ran a 300-byte name under a file prefix: {native:?}");
                };
                assert_eq!(errno_of(error), Some(rustix::io::Errno::NOTDIR), "{error}");
                assert_eq!(
                    refused(resolved),
                    exhausted(&overlong, &file.join(&overlong))
                );
            }
            _ => record("overlong-under-file-prefix", &native, &resolved),
        }
    });

    // Missing, then existing: the miss is walked past and the existing
    // directory's ENAMETOOLONG stops the search.
    let missing_then_existing =
        OsString::from(format!("{}:{}", nowhere.display(), controls.display()));
    collecting(
        &mut failures,
        "control:overlong-missing-then-existing",
        || {
            let native = oracle(
                Command::new(&overlong)
                    .arg("--list")
                    .stdin(Stdio::null())
                    .env("PATH", &missing_then_existing),
                "control:overlong-missing-then-existing",
            );
            let resolved = resolve_executable_in(&overlong, Some(missing_then_existing.clone()));
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
                    assert_eq!(refused(resolved), terminal(&controls.join(&overlong)));
                }
                _ => record("overlong-missing-then-existing", &native, &resolved),
            }
        },
    );

    // At the working directory (`PATH=""`): the bare 300-byte name is
    // ENAMETOOLONG to the kernel, on which native stops — and a native
    // stop at the cwd candidate is preserved as that cause, not renamed
    // to the cwd reason.
    collecting(&mut failures, "control:overlong-at-cwd", || {
        let native = oracle(
            Command::new(&overlong)
                .arg("--list")
                .stdin(Stdio::null())
                .env("PATH", ""),
            "control:overlong-at-cwd",
        );
        let resolved = resolve_executable_in(&overlong, Some(OsString::new()));
        match LIBRARY {
            Library::Glibc => {
                let Outcome::Failed(error) = &native else {
                    panic!("the native child ran a 300-byte name from cwd: {native:?}");
                };
                assert_eq!(
                    errno_of(error),
                    Some(rustix::io::Errno::NAMETOOLONG),
                    "{error}"
                );
                assert_eq!(refused(resolved), terminal(Path::new(&overlong)));
            }
            _ => record("overlong-at-cwd", &native, &resolved),
        }
    });

    // With only an oversized component there is no candidate at all,
    // so nothing is measured: glibc breaks out of its loop with errno
    // UNTOUCHED (`posix/execvpe.c` 121–122, 160–168), and the errno the
    // child reports is whatever this thread had set before the fork.
    // That is proved rather than described: the errno is planted by a
    // failing `metadata` call — ENOENT, then ENOTDIR — and the child
    // reports exactly the planted value each time, which no attempted
    // `execve` would have left standing.
    let skipped = OsString::from("x".repeat(5000));
    for (what, plant, planted) in [
        (
            "overlong-no-candidate",
            nowhere.join("plant"),
            rustix::io::Errno::NOENT,
        ),
        (
            "overlong-no-candidate-notdir",
            file.join("plant"),
            rustix::io::Errno::NOTDIR,
        ),
    ] {
        collecting(&mut failures, &format!("control:{what}"), || {
            let expected = fs::metadata(&plant).unwrap_err();
            assert_eq!(errno_of(&expected), Some(planted));
            let native = oracle(
                Command::new(&overlong)
                    .arg("--list")
                    .stdin(Stdio::null())
                    .env("PATH", &skipped),
                &format!("control:{what}"),
            );
            let resolved = resolve_executable_in(&overlong, Some(skipped.clone()));
            match LIBRARY {
                Library::Glibc => {
                    let Outcome::Failed(error) = &native else {
                        panic!("{what}: the native child ran a 300-byte name with no candidate: {native:?}");
                    };
                    assert_eq!(
                        errno_of(error),
                        Some(planted),
                        "{what}: nothing was attempted, so the planted errno stands: {error}"
                    );
                    assert_eq!(
                        refused(resolved),
                        format!(
                        "the DSH layout is unreadable: '{overlong}' is not on PATH (the search \
                         attempted no candidate: every component was skipped as longer than the \
                         buffer the platform builds one in)"
                    )
                    );
                }
                _ => record(what, &native, &resolved),
            }
        });
    }

    // The same name as an explicit path: `execve` itself answers
    // ENAMETOOLONG, and the resolver's one candidate stops by that cause.
    //
    // A DIRECT name is answered by `execve` on every arm, so the
    // terminal refusal is the same on every arm: this control is a
    // library-independent expectation, not glibc's (the Apple-arm audit,
    // 2026-09-21; `the_lookup_rule_is_each_librarys_own_switch_arm_by_arm`
    // proves the same refusal under each injected library on Linux).
    let spelled = controls.join(&overlong);
    collecting(&mut failures, "control:overlong-explicit-path", || {
        let native = oracle(
            Command::new(&spelled).arg("--list").stdin(Stdio::null()),
            "control:overlong-explicit-path",
        );
        let resolved = resolve_executable(spelled.to_str().unwrap());
        let Outcome::Failed(error) = &native else {
            panic!("the native child ran an overlong explicit path: {native:?}");
        };
        assert_eq!(
            errno_of(error),
            Some(rustix::io::Errno::NAMETOOLONG),
            "{error}"
        );
        assert_eq!(refused(resolved), terminal(&spelled));
    });

    // The NAME_MAX boundary itself: 256 bytes is one more than the
    // kernel takes for a component, refused by the kernel under the
    // existing controls directory even though nothing of that name is
    // there — and never by a length rule of the resolver's own.
    let boundary = "y".repeat(256);
    collecting(&mut failures, "control:name-max-boundary", || {
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
                assert_eq!(refused(resolved), terminal(&controls.join(&boundary)));
            }
            _ => record("name-max-boundary", &native, &resolved),
        }
    });

    // The valid-length positive: a 255-byte name that EXISTS on the
    // search runs natively and is selected, so length continuation and
    // length refusal are both measured against a name that runs.
    let valid = "z".repeat(255);
    collecting(&mut failures, "control:valid-length-name", || {
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
    });

    // Exhaustion keeps the LAST candidate's cause, not a relabelled
    // NotFound: missing then file ends in ENOTDIR, file then missing in
    // ENOENT (design D10 §3).
    for (what, first, second) in [
        ("exhaustion-missing-then-file", &nowhere, &file),
        ("exhaustion-file-then-missing", &file, &nowhere),
    ] {
        let path = OsString::from(format!("{}:{}", first.display(), second.display()));
        collecting(&mut failures, &format!("control:{what}"), || {
            let native = oracle(
                Command::new("dsh")
                    .arg("--list")
                    .stdin(Stdio::null())
                    .env("PATH", &path),
                &format!("control:{what}"),
            );
            let resolved = resolve_executable_in("dsh", Some(path.clone()));
            match LIBRARY {
                Library::Glibc => {
                    let Outcome::Failed(error) = &native else {
                        panic!("{what}: the native child ran a dsh: {native:?}");
                    };
                    let expected = fs::metadata(second.join("dsh")).unwrap_err();
                    assert_eq!(
                        error.raw_os_error(),
                        expected.raw_os_error(),
                        "{what}: native reports the last candidate's cause: {error}"
                    );
                    assert_eq!(refused(resolved), exhausted("dsh", &second.join("dsh")));
                }
                _ => record(what, &native, &resolved),
            }
        });
    }

    // The sealed-directory controls' EXPLICIT form: the `PATH` set on the
    // `Command` and handed to the resolver's `execvp` operation. The
    // parent staged the fixtures and restores their permission.
    let sealed = Sealed::under(&root);
    for what in SEALED {
        let id = format!("control:{what}");
        collecting(&mut failures, &id, || {
            let path = sealed.path(what);
            sealed.control(what, &id, Command::new("dsh").env("PATH", &path), || {
                resolve_executable_in("dsh", Some(path.clone()))
            });
        });
    }

    // The default-search positive AO requires beside the all-negative
    // absent-PATH cells: `sh` runs with PATH removed, and the resolver
    // selects the very file it ran, with a same-name cwd decoy present.
    collecting(&mut failures, "control:default-search-sh", || {
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
    });
    println!("\nmatrix-tally: 0 0 0 0 0 0");
    report("the controls", failures);
}

/// One sealed-directory control's INHERITED form, in a child whose own
/// environment carries the `PATH` it searches: production's invocation,
/// `Command::new` with no environment change (`posix_spawnp`), beside
/// production's resolver reading the same environment.
fn child_sealed(what: &str) {
    let cwd = std::env::current_dir().unwrap();
    let root = cwd.parent().unwrap().to_path_buf();
    let sealed = Sealed::under(&root);
    assert_eq!(
        std::env::var_os("PATH"),
        Some(sealed.path(what)),
        "the parent staged {what}'s PATH in this child's environment"
    );
    let mut failures: Vec<String> = Vec::new();
    let id = format!("control:{what}-inherited");
    collecting(&mut failures, &id, || {
        sealed.control(what, &id, &mut Command::new("dsh"), || {
            resolve_executable("dsh")
        });
    });
    println!("\nmatrix-tally: 0 0 0 0 0 0");
    report(&id, failures);
}

#[test]
fn native_executable_resolution_matches_command_matrix() {
    match std::env::var(CASE) {
        Ok(case) if case == "controls" => child_controls(),
        Ok(case) => match (case.strip_prefix("sealed:"), case.strip_suffix(":removed")) {
            (Some(what), _) => child_sealed(what),
            (None, Some(index)) => child_layout(index.parse().unwrap(), true),
            (None, None) => child_layout(case.parse().unwrap(), false),
        },
        Err(_) => parent(),
    }
}
