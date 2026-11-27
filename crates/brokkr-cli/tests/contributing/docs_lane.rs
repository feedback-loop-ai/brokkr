//! The docs lane (#1295): a pull request that changes only documentation
//! runs the tests that read documentation, which scripts/docs-lane-tests.txt
//! pins, in place of the whole matrix, and every required check still
//! passes on a result, never on a skip.
//!
//! Held here: the list is complete, so a test file that reads
//! documentation it does not name is in it; and the verdicts read the
//! classifier and the docs lane, so no required check can pass because a
//! job it judges was skipped.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use super::{read_condition, workspace, MAIN_REQUIRES};
use crate::workflow;

/// The docs lane's pinned tests.
const LIST: &str = "scripts/docs-lane-tests.txt";

/// One entry of the list: the package, the test target and the libtest
/// filter it runs, and the files that filter runs whole, each path with the
/// tracked files it matches as the classifier's `:(exclude,literal)`
/// pathspec reads it, so the guard holds the very files the classifier
/// stops searching.
#[derive(Debug)]
struct Entry {
    package: String,
    target: String,
    filter: String,
    paths: Vec<(String, Vec<String>)>,
}

impl Entry {
    /// Whether the classifier skips `file` for one of this entry's paths.
    fn covers(&self, file: &str) -> bool {
        self.paths
            .iter()
            .any(|(_, matched)| matched.iter().any(|path| path == file))
    }
}

/// The list's entries, comments and blank lines skipped; a line short of a
/// path is refused.
fn entries(root: &Path, list: &str) -> Vec<Entry> {
    list.lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            assert!(fields.len() >= 4, "{LIST}: `{line}` names no file");
            let matched =
                |path: &str| crate::tracked_files::tracked(root, &[&format!(":(literal){path}")]);
            Entry {
                package: fields[0].to_string(),
                target: fields[1].to_string(),
                filter: fields[2].to_string(),
                paths: fields[3..]
                    .iter()
                    .map(|path| (path.to_string(), matched(path)))
                    .collect(),
            }
        })
        .collect()
}

/// How a test file reads documentation: a `.md` under docs/ or at the top
/// level, the only files the classifier can call documentation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Reads {
    /// It names each such file it reads, so the classifier, which refuses a
    /// file named outside the docs lane, refuses each unless it is listed.
    Names,
    /// It reads such files without naming them: a directory under docs/, a
    /// pattern, every `.md` or every tracked file. It must be listed.
    Enumerates,
}

/// The pages the classifier can call documentation: the tracked `.md`
/// files under docs/ and at the top level.
struct Pages {
    top_level: Vec<String>,
    under_docs: Vec<String>,
}

impl Pages {
    fn of(root: &Path) -> Self {
        Self {
            top_level: crate::tracked_files::tracked(root, &[":(glob)*.md"]),
            under_docs: crate::tracked_files::tracked(root, &[":(glob)docs/**/*.md"]),
        }
    }

    /// What one string literal on `line` reads, when it reads a page: a
    /// page by name, or a directory holding one, a pattern, or `.md` itself
    /// without one. A bare `docs` is a directory only where the line joins
    /// or lists a path, since elsewhere it is a name.
    ///
    /// It fails closed: a git pathspec's magic is read past, and a pattern
    /// or a format that could reach a `.md` file or docs/ enumerates.
    fn literal_reads(&self, literal: &str, line: &str) -> Option<Reads> {
        let (magic, literal) = match literal.strip_prefix(":(") {
            Some(magic) => (true, magic.split_once(')').map_or(magic, |(_, rest)| rest)),
            None => (false, literal),
        };
        let bare = literal.trim_start_matches(['.', '/']);
        if matches!(literal, "md" | ".md") || pattern_enumerates(bare, magic, line) {
            return Some(Reads::Enumerates);
        }
        if self.top_level.iter().any(|name| bare == name) {
            return Some(Reads::Names);
        }
        self.under_docs_reads(bare, line)
    }

    /// What a literal that is no pattern reads under docs/: a page by name,
    /// or docs/ itself, a directory holding a page, or a placeholder in it.
    fn under_docs_reads(&self, bare: &str, line: &str) -> Option<Reads> {
        let rest = match bare.strip_prefix("docs") {
            Some("") => {
                return ["join(", "read_dir(", "tracked("]
                    .iter()
                    .any(|path| line.contains(path))
                    .then_some(Reads::Enumerates)
            }
            Some(rest) => rest.strip_prefix('/')?,
            None => return None,
        };
        if rest.trim_end_matches('/').is_empty() || rest.contains(['{', '*']) {
            return Some(Reads::Enumerates);
        }
        if rest.ends_with(".md") {
            return Some(Reads::Names);
        }
        let directory = format!("docs/{}/", rest.trim_end_matches('/'));
        self.under_docs
            .iter()
            .any(|page| page.starts_with(&directory))
            .then_some(Reads::Enumerates)
    }

    /// How a file's code reads pages, its comments skipped: the most any
    /// line reads, a line's literals split at its double quotes, and every
    /// tracked file read whole an enumeration, `tracked_text` named in any
    /// form (a `use` of it, an alias) among them.
    ///
    /// It fails closed on a bare `docs` in a file that reaches the
    /// workspace: there it enumerates wherever it stands, since a constant
    /// or an array can carry it to a `join` on another line (a reader that
    /// keeps the directory in a constant would otherwise pass unlisted). A
    /// file that never reaches the workspace reads no page through it.
    fn file_reads(&self, text: &str) -> Option<Reads> {
        let code: Vec<&str> = text
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect();
        let reaches = code
            .iter()
            .any(|line| WORKSPACE_ROOTS.iter().any(|root| line.contains(root)));
        code.into_iter()
            .flat_map(|line| {
                let declares = ["mod ", "#[path"]
                    .iter()
                    .any(|head| line.trim_start().starts_with(head));
                let whole_tree = ((line.contains("tracked_text") && !declares)
                    || (line.contains("tracked(") && line.contains(", &[])")))
                .then_some(Reads::Enumerates);
                line.split('"')
                    .skip(1)
                    .step_by(2)
                    .filter_map(|literal| {
                        self.literal_reads(literal, line).or_else(|| {
                            (reaches && bare_docs(literal)).then_some(Reads::Enumerates)
                        })
                    })
                    .chain(whole_tree)
                    .collect::<Vec<_>>()
            })
            .max()
    }
}

/// What a file names to reach the workspace's own tree: its manifest
/// directory, or the test crates' helpers that return the workspace root.
const WORKSPACE_ROOTS: [&str; 3] = ["CARGO_MANIFEST_DIR", "workspace(", "repo_root("];

/// Whether a literal is docs/ itself, its pathspec magic and leading or
/// trailing slashes read past.
fn bare_docs(literal: &str) -> bool {
    let literal = match literal.strip_prefix(":(") {
        Some(magic) => magic.split_once(')').map_or(magic, |(_, rest)| rest),
        None => literal,
    };
    literal.trim_start_matches(['.', '/']).trim_end_matches('/') == "docs"
}

/// Whether a literal, its pathspec magic read past, is a pattern that can
/// reach a page: a glob that ends in `md` or reaches into docs/; one of
/// every file where the line lists tracked files (`magic`, the literal was
/// a pathspec, says it does); a format whose placeholder is a root above
/// docs/.
fn pattern_enumerates(bare: &str, magic: bool, line: &str) -> bool {
    let lists = magic
        || ["tracked(", "ls-files"]
            .iter()
            .any(|read| line.contains(read));
    let glob = bare.contains('*')
        && (bare.ends_with("md")
            || bare.contains("docs")
            || (lists && bare.trim_matches(['*', '/']).is_empty()));
    glob || (bare.starts_with('{') && bare.contains("}/docs"))
}

/// The package, the test target and the module path a test file's tests
/// run under: `it`'s modules by their files (`#[path]`'d support aside), a
/// file of its own target at that target's root, and a library's modules
/// under `lib`.
fn test_home(root: &Path, file: &str) -> (String, String, String) {
    let parts: Vec<&str> = file.split('/').collect();
    let (package, kind, rest) = (parts[1].to_string(), parts[2], &parts[3..]);
    let mut segments: Vec<&str> = rest
        .iter()
        .map(|part| part.strip_suffix(".rs").unwrap_or(part))
        .filter(|part| !matches!(*part, "main" | "mod"))
        .collect();
    if kind == "src" {
        segments.retain(|part| *part != "lib");
        return (package, "lib".to_string(), segments.join("::"));
    }
    let it = std::fs::read_to_string(root.join(format!("crates/{package}/tests/it.rs")))
        .unwrap_or_default();
    let declared = it.lines().any(|line| {
        line.trim() == format!("mod {};", segments[0])
            || line.trim() == format!("#[path = \"{}/main.rs\"]", segments[0])
    });
    match declared {
        true => (package, "test:it".to_string(), segments.join("::")),
        false => (package, format!("test:{}", segments[0]), String::new()),
    }
}

/// The tracked `.rs` files of every package's tests/ and src/, with their
/// text: a support module, or a module of src/ that holds no `#[test]` of
/// its own, included, so one that reads pages is held to its callers.
fn test_files(root: &Path) -> Vec<(String, String)> {
    crate::tracked_files::tracked(root, &["crates/"])
        .into_iter()
        .filter(|path| {
            path.ends_with(".rs") && matches!(path.split('/').nth(2), Some("tests" | "src"))
        })
        .filter_map(|path| {
            let text = std::fs::read_to_string(root.join(&path)).ok()?;
            Some((path, text))
        })
        .collect()
}

/// The test files the rule above calls readers of unnamed pages that read
/// only files the classifier refuses to call docs, each with what it reads
/// instead.
const EXEMPT: [(&str, &str); 13] = [
    (
        "crates/brokkr-protocol/src/adapters.rs",
        "`.md` is the suffix of a staged prompt's temporary file",
    ),
    (
        "crates/brokkr-cli/src/doctor/tests.rs",
        "`docs` is a scratch realm's directory, joined to a temporary directory",
    ),
    (
        "crates/brokkr-cli/tests/capability_broker/enabled.rs",
        "`docs` is a stub MCP server's name in its handshake",
    ),
    (
        "crates/brokkr-cli/tests/realms.rs",
        "it links the workspace's docs/ so a World loads docs/house-rules.md alone, which realms.json names, so the classifier refuses it",
    ),
    (
        "crates/brokkr-cli/tests/init_doctor.rs",
        "docs/guides/quickstart.md alone, whose file name TRANSCRIPTS names, so the classifier refuses it",
    ),
    (
        "crates/brokkr-core/tests/committed_journals.rs",
        "the `.ndjson` journals under docs/evidence alone, no `.md` page",
    ),
    (
        "crates/brokkr-runtime/src/agents/charter_data/tests.rs",
        "`docs` is a capability's name in a charter's asks",
    ),
    (
        "crates/brokkr-runtime/src/capabilities/tests/dialect_policy.rs",
        "`docs` is a dialect's name, its file under a temporary root",
    ),
    (
        "crates/brokkr-runtime/src/engine/artifact_gate_tests.rs",
        "`docs` is a directory of a temporary tree",
    ),
    (
        "crates/brokkr-runtime/src/engine/capability_tests/provision_tests/brokered_tests.rs",
        "`docs` is a panel member's name",
    ),
    (
        "crates/brokkr-runtime/src/engine/capability_tests/provision_tests/brokered_tests/writers_tests.rs",
        "`docs` is a link under a fixture's root",
    ),
    (
        "crates/brokkr-runtime/tests/landing_shape.rs",
        "`docs/decisions` is a directory of a scratch repository",
    ),
    (
        "crates/brokkr-runtime/tests/review_scope.rs",
        "every .md under agents/charters and recipes/*/roles, none under docs/ or at the top",
    ),
];

/// Where a support module that holds no test and reads documentation it
/// does not name escapes the lane: each file of its package's tests/ that
/// reaches it, by its name and `::`, is listed. One reached by nothing is a
/// reader the lane cannot see run, and is refused too.
fn support_problems<'a>(
    file: &str,
    files: &'a [(String, String)],
    listed: &dyn Fn(&str) -> Option<&'a Entry>,
) -> Vec<String> {
    let stem = Path::new(file).file_stem().and_then(|stem| stem.to_str());
    let tests = file.split('/').take(3).collect::<Vec<_>>().join("/") + "/";
    let reach = stem.map(|stem| format!("{stem}::")).unwrap_or_default();
    let reaches = |text: &str| {
        text.match_indices(&reach).any(|(at, _)| {
            !text[..at].ends_with(|before: char| before.is_alphanumeric() || before == '_')
        })
    };
    let callers: Vec<&str> = files
        .iter()
        .filter(|(caller, text)| {
            caller != file && caller.starts_with(&tests) && !reach.is_empty() && reaches(text)
        })
        .map(|(caller, _)| caller.as_str())
        .collect();
    if callers.is_empty() {
        return vec![format!(
            "{file} reads documentation it does not name, and no test of its package reaches it"
        )];
    }
    callers
        .into_iter()
        .filter(|caller| listed(caller).is_none())
        .map(|caller| {
            format!("{caller} reaches {file}, which reads documentation it does not name, and {LIST} does not run it")
        })
        .collect()
}

/// Where one entry fails to run what it names: each path matches a tracked
/// file, and each file it matches, which the classifier then never
/// searches, is a test file that runs under the entry's own filter.
fn entry_problems(root: &Path, files: &[(String, String)], entry: &Entry) -> Vec<String> {
    let mut problems = Vec::new();
    for (path, matched) in &entry.paths {
        if matched.is_empty() {
            problems.push(format!(
                "{LIST} names {path}, which matches no tracked file"
            ));
        }
        for file in matched {
            if !files.iter().any(|(test, _)| test == file) {
                problems.push(format!("{LIST} names {file}, which is no test file"));
                continue;
            }
            let (package, target, module) = test_home(root, file);
            if (package.as_str(), target.as_str()) != (&entry.package, &entry.target)
                || !format!("{module}::").starts_with(entry.filter.as_str())
            {
                problems.push(format!(
                    "{file} runs as {package} {target} {module}, not under its entry's `{} {} {}`",
                    entry.package, entry.target, entry.filter
                ));
            }
        }
    }
    problems
}

/// Where an unlisted file escapes the lane: one that reads documentation it
/// does not name is exempt, or a support module whose callers are all
/// listed; and an exemption is a file that still enumerates.
fn reader_problems<'a>(
    (file, text): &(String, String),
    reads: Option<Reads>,
    files: &'a [(String, String)],
    listed: &dyn Fn(&str) -> Option<&'a Entry>,
) -> Vec<String> {
    let exempt = EXEMPT.iter().any(|(path, _)| path == file);
    match (reads, exempt) {
        (Some(Reads::Enumerates), false) if !text.contains("#[test]") => {
            support_problems(file, files, listed)
        }
        (Some(Reads::Enumerates), false) => vec![format!(
            "{file} reads documentation it does not name, and {LIST} does not run it"
        )],
        (Some(Reads::Enumerates), true) | (Some(Reads::Names) | None, false) => Vec::new(),
        (Some(Reads::Names) | None, true) => {
            vec![format!("{file} is exempt and no longer reads unnamed docs")]
        }
    }
}

/// Where the docs lane fails to run a test that reads documentation, one
/// problem a line: every entry runs what it names; every exemption is a
/// file the guard reads; every unlisted file that
/// reads documentation it does not name is exempt or reached only from
/// listed files; and no unlisted file of brokkr-cli's tests reaches a
/// listed `it` module's helpers, whose reads would escape the lane.
fn lane_problems(root: &Path, files: &[(String, String)], list: &str) -> Vec<String> {
    let pages = Pages::of(root);
    let entries = entries(root, list);
    let listed = |file: &str| entries.iter().find(|entry| entry.covers(file));
    let mut problems: Vec<String> = entries
        .iter()
        .flat_map(|entry| entry_problems(root, files, entry))
        .collect();
    for (path, _) in EXEMPT {
        if !files.iter().any(|(file, _)| file == path) {
            problems.push(format!("{path} is exempt and is no file the guard reads"));
        }
    }
    for pair in files.iter().filter(|(file, _)| listed(file).is_none()) {
        problems.extend(reader_problems(
            pair,
            pages.file_reads(&pair.1),
            files,
            &listed,
        ));
    }
    let modules: BTreeSet<&str> = entries
        .iter()
        .filter(|entry| entry.package == "brokkr-cli" && entry.target == "test:it")
        .filter_map(|entry| entry.filter.strip_suffix("::"))
        .filter(|module| !module.contains("::"))
        .collect();
    let unlisted = files
        .iter()
        .filter(|(file, _)| file.starts_with("crates/brokkr-cli/tests/") && listed(file).is_none());
    for (file, text) in unlisted {
        for module in &modules {
            if text.contains(&format!("crate::{module}::")) {
                problems.push(format!(
                    "{file} reaches crate::{module}, which only the docs lane runs whole"
                ));
            }
        }
    }
    problems
}

/// Every test file that reads documentation it does not name is in the
/// docs lane's list or exempt with its reason, and every listed file runs
/// under its entry's filter. Altered trees are the control: an entry
/// dropped, an unlisted reader of a docs/ directory added, a support module
/// that reads every `.md` by a pathspec glob and that nothing reaches, the
/// same module reached by an unlisted test, an entry's filter that misses
/// its files, an entry that names a directory its filter does not run, a
/// page or a missing file, and a module of src/ with no test of its own
/// that reads every page.
#[test]
fn the_docs_lane_runs_every_test_that_reads_documentation() {
    let root = workspace();
    let files = test_files(&root);
    let list = std::fs::read_to_string(root.join(LIST)).unwrap();
    assert_eq!(lane_problems(&root, &files, &list), Vec::<String>::new());

    let dropped: String = list
        .lines()
        .filter(|line| !line.starts_with("brokkr-cli test:it doc_truth::"))
        .map(|line| format!("{line}\n"))
        .collect();
    assert_ne!(dropped, list, "the list runs doc_truth");
    assert!(
        lane_problems(&root, &files, &dropped).contains(&format!(
            "crates/brokkr-cli/tests/doc_truth.rs reads documentation it does not name, and {LIST} does not run it"
        )),
        "a dropped entry passes"
    );

    // The panel's shape (1295b, ruling 9): a reader that keeps docs/ in a
    // constant, whose `join` stands on another line. Its entry dropped, the
    // guard refuses it.
    let unlisted: String = list
        .lines()
        .filter(|line| !line.starts_with("brokkr-cli test:it layering::test_targets::"))
        .map(|line| format!("{line}\n"))
        .collect();
    assert_ne!(unlisted, list, "the list runs layering::test_targets");
    assert!(
        lane_problems(&root, &files, &unlisted).contains(&format!(
            "crates/brokkr-cli/tests/layering/test_targets.rs reads documentation it does not name, and {LIST} does not run it"
        )),
        "a reader holding docs/ in a constant passes unlisted"
    );

    let reader = "crates/brokkr-cli/tests/new_reader.rs";
    for (text, problem) in [
        (
            "#[test]\nfn reads() { walk(workspace().join(\"docs/guides\")); }\n",
            format!("{reader} reads documentation it does not name, and {LIST} does not run it"),
        ),
        (
            "const TREE: &str = \"docs\";\n#[test]\nfn reads() { walk(workspace().join(TREE)); }\n",
            format!("{reader} reads documentation it does not name, and {LIST} does not run it"),
        ),
        (
            "pub fn pages() { tracked(&root, &[\":(glob)**/*.md\"]); }\n",
            format!(
                "{reader} reads documentation it does not name, and no test of its package reaches it"
            ),
        ),
    ] {
        let mut added = files.clone();
        added.push((reader.to_string(), text.to_string()));
        assert_eq!(lane_problems(&root, &added, &list), [problem], "{text}");
    }
    let mut reached = files.clone();
    reached.push((
        "crates/brokkr-cli/tests/support/numbered_pages.rs".to_string(),
        "pub fn all() { tracked(&root, &[\":(glob)docs/**/*.md\"]); }\n".to_string(),
    ));
    reached.push((
        "crates/brokkr-cli/tests/new_reader.rs".to_string(),
        "#[test]\nfn reads() { crate::numbered_pages::all(); }\n".to_string(),
    ));
    assert_eq!(
        lane_problems(&root, &reached, &list),
        [format!(
            "{reader} reaches crates/brokkr-cli/tests/support/numbered_pages.rs, which reads documentation it does not name, and {LIST} does not run it"
        )]
    );

    let missed = list.replace(
        "brokkr-cli test:it doc_truth::",
        "brokkr-cli test:it doc_truths::",
    );
    assert_ne!(missed, list);
    assert!(
        lane_problems(&root, &files, &missed).contains(&
            "crates/brokkr-cli/tests/doc_truth.rs runs as brokkr-cli test:it doc_truth, not under its entry's `brokkr-cli test:it doc_truths::`".to_string()
        ),
        "a filter that misses its files passes"
    );

    // An entry that names more than its filter runs: the classifier stops
    // searching every file a path matches, so each must run under it.
    let entry = "brokkr-cli test:it contributing:: crates/brokkr-cli/tests/contributing.rs";
    for (path, problem) in [
        (
            "crates/brokkr-runtime/tests",
            "crates/brokkr-runtime/tests/adoption.rs runs as brokkr-runtime test:it adoption, not under its entry's `brokkr-cli test:it contributing::`".to_string(),
        ),
        (
            "docs/guides/provider-adapters.md",
            format!("{LIST} names docs/guides/provider-adapters.md, which is no test file"),
        ),
        (
            "crates/brokkr-cli/tests/gone.rs",
            format!("{LIST} names crates/brokkr-cli/tests/gone.rs, which matches no tracked file"),
        ),
    ] {
        let widened = list.replace(entry, &format!("{entry} {path}"));
        assert_ne!(widened, list);
        assert!(
            lane_problems(&root, &files, &widened).contains(&problem),
            "an entry naming {path} passes"
        );
    }

    // A module of src/ with no test of its own that reads every page.
    let mut support = files.clone();
    let module = "crates/brokkr-core/src/numbered_pages.rs";
    support.push((
        module.to_string(),
        "pub fn all() { tracked(&root, &[\":(glob)docs/**/*.md\"]); }\n".to_string(),
    ));
    assert_eq!(
        lane_problems(&root, &support, &list),
        [format!(
            "{module} reads documentation it does not name, and no test of its package reaches it"
        )]
    );
}

/// What a literal reads, case by case: a named page, a directory with a
/// page in it, a pattern, an extension, a top-level page, and what reads no
/// page: a file of another kind, a directory of none, a name.
#[test]
fn a_literal_names_or_enumerates_documentation() {
    let pages = Pages {
        top_level: vec!["README.md".to_string()],
        under_docs: vec!["docs/guides/quickstart.md".to_string()],
    };
    let join = "root.join(…)";
    for (literal, line, reads) in [
        ("docs/guides/quickstart.md", "", Some(Reads::Names)),
        ("../../docs/guides/quickstart.md", "", Some(Reads::Names)),
        ("README.md", "", Some(Reads::Names)),
        ("docs", join, Some(Reads::Enumerates)),
        ("docs/", "", Some(Reads::Enumerates)),
        ("docs/guides/", "", Some(Reads::Enumerates)),
        ("docs/guides", "", Some(Reads::Enumerates)),
        ("docs/{name}.md", "", Some(Reads::Enumerates)),
        ("*.md", "", Some(Reads::Enumerates)),
        ("**/*.md", "", Some(Reads::Enumerates)),
        (":(glob)docs/**/*.md", "", Some(Reads::Enumerates)),
        (":(glob)**", "", Some(Reads::Enumerates)),
        ("{root}/docs", "", Some(Reads::Enumerates)),
        ("md", "", Some(Reads::Enumerates)),
        ("**/*.rs", "", None),
        (":(glob)crates/**/*.rs", "", None),
        ("docs", "member(…)", None),
        ("docs/evidence/adapters", "", None),
        ("docs/examples/adapters/qoder.json", "", None),
        ("opt/docs/bin/server", "", None),
        ("agents/README.md", "", None),
        ("documents", "", None),
    ] {
        assert_eq!(pages.literal_reads(literal, line), reads, "{literal}");
    }
    assert_eq!(
        pages.file_reads("// \"docs/guides\"\nlet x = tracked_text::texts(&root);\n"),
        Some(Reads::Enumerates)
    );
    assert_eq!(
        pages.file_reads("use crate::tracked_text as texts;\n"),
        Some(Reads::Enumerates)
    );
    assert_eq!(pages.file_reads("// reads \"docs/guides\"\n"), None);
    // A bare `docs` enumerates anywhere in a file that reaches the
    // workspace, and names nothing in one that does not.
    for (text, reads) in [
        (
            "const TREE: &str = \"docs\";\nlet root = workspace().join(TREE);\n",
            Some(Reads::Enumerates),
        ),
        (
            "for tree in [\"docs/\", \"dialects\"] {}\nlet root = env!(\"CARGO_MANIFEST_DIR\");\n",
            Some(Reads::Enumerates),
        ),
        ("let member = \"docs\";\n", None),
        ("// workspace()\nlet member = \"docs\";\n", None),
    ] {
        assert_eq!(pages.file_reads(text), reads, "{text}");
    }
}

/// One verdict job's `run:` block, parsed structurally from ci.yml.
fn verdict_block(wf: &workflow::Workflow, id: &str) -> String {
    wf.require_job("ci.yml", id)
        .unwrap()
        .steps
        .iter()
        .find_map(|s| s.run.as_deref())
        .unwrap_or_else(|| panic!("{id} has no run block"))
        .to_string()
}

/// Whether `block` passes under bash -e, the shell a step runs, with `env`.
fn passes(block: &str, env: &[(&str, &str)]) -> bool {
    Command::new("bash")
        .args(["-e", "-c", block])
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap())
        .envs(env.iter().copied())
        .status()
        .expect("bash")
        .success()
}

/// The suite's and the coverage gate's verdicts, each the variable its
/// other job's result is in, and that job.
const VERDICT_JOBS: [(&str, &str, &str); 2] = [
    ("engine", "MACHINES", "engine-machines"),
    ("coverage-gate", "REPORT", "coverage"),
];

/// Each verdict passes on a result, never on a skip: run under bash with
/// every combination of the classifier's result and lane, the docs lane's
/// result and the other job's, it passes exactly when the classifier
/// succeeded and the job its lane names succeeded. Its job runs `always()`
/// on every job its verdict reads, and names each result where its block
/// reads it. A verdict softened to pass on a skipped docs lane is the
/// control.
#[test]
fn every_verdict_passes_on_the_lanes_own_result_never_on_a_skip() {
    let ci = std::fs::read_to_string(workspace().join(".github/workflows/ci.yml")).unwrap();
    let wf = workflow::Workflow::parse(&ci).expect("ci.yml parses");
    let results = ["success", "failure", "cancelled", "skipped", ""];
    let cases: Vec<[&str; 4]> = results
        .iter()
        .flat_map(|classified| ["docs", "full", ""].map(|lane| (*classified, lane)))
        .flat_map(|(classified, lane)| results.map(|docs| (classified, lane, docs)))
        .flat_map(|(classified, lane, docs)| results.map(|other| [classified, lane, docs, other]))
        .collect();
    let truth = |block: &str, other: &str| {
        let mut wrong = Vec::new();
        for [classified, lane, docs, result] in &cases {
            let env = [
                ("CLASSIFIED", *classified),
                ("LANE", *lane),
                ("DOCS", *docs),
                (other, *result),
            ];
            let expected = *classified == "success"
                && match *lane {
                    "docs" => *docs == "success",
                    "full" => *result == "success",
                    _ => false,
                };
            if passes(block, &env) != expected {
                wrong.push(format!("{env:?}"));
            }
        }
        wrong
    };
    for (id, other, job) in VERDICT_JOBS {
        let block = verdict_block(&wf, id);
        assert_eq!(truth(&block, other), Vec::<String>::new(), "{id}'s verdict");
        let parsed = wf.require_job("ci.yml", id).unwrap();
        assert_eq!(
            parsed.needs,
            ["classify", "docs-lane", job],
            "{id} needs mismatch"
        );
        assert!(
            parsed
                .if_cond
                .as_deref()
                .is_some_and(|c| c.starts_with("always()")),
            "{id} does not run whatever its needs ended with"
        );
        let run_step = parsed
            .steps
            .iter()
            .find(|s| s.run.is_some())
            .expect("verdict run step");
        for (name, value) in [
            ("CLASSIFIED", "needs.classify.result".to_string()),
            ("LANE", "needs.classify.outputs.lane".to_string()),
            ("DOCS", "needs.docs-lane.result".to_string()),
            (other, format!("needs.{job}.result")),
        ] {
            let expected = format!("${{{{ {value} }}}}");
            assert_eq!(
                run_step.env_value(name),
                Some(expected.as_str()),
                "{id} does not set {name}: {expected}"
            );
        }
    }
    let softened = verdict_block(&wf, "engine").replace(
        "docs) test \"$DOCS\" = success ;;",
        "docs) test \"$DOCS\" != failure ;;",
    );
    assert!(
        !truth(&softened, "MACHINES").is_empty(),
        "a verdict passing a skipped docs lane passes"
    );
}

/// The heavy machines start on no docs change, and the docs lane on
/// nothing else: each machine job needs the classifier and its condition
/// refuses the docs lane, and the docs lane's condition is the lane itself.
/// The docs lane runs its list and nothing else, with no failure let
/// through.
#[test]
fn the_heavy_machines_do_not_start_on_a_docs_change() {
    let ci = std::fs::read_to_string(workspace().join(".github/workflows/ci.yml")).unwrap();
    let wf = workflow::Workflow::parse(&ci).expect("ci.yml parses");
    for (id, condition) in [
        (
            "engine-machines",
            "github.event_name != 'push' && needs.classify.outputs.lane != 'docs'",
        ),
        ("coverage-machines", "needs.classify.outputs.lane != 'docs'"),
        ("docs-lane", "needs.classify.outputs.lane == 'docs'"),
    ] {
        let job = wf.require_job("ci.yml", id).unwrap();
        assert_eq!(job.needs, ["classify"], "{id}'s needs");
        assert_eq!(job.if_cond.as_deref(), Some(condition), "{id}'s condition");
    }
    assert_eq!(
        wf.require_job("ci.yml", "classify").unwrap().if_cond,
        None,
        "the classifier runs on every event"
    );
    assert_eq!(lane_runs(&wf), ["bash scripts/docs-lane.sh run"]);
}

/// What the docs lane's job runs: each step's `run:` value, and any
/// `continue-on-error` on the job or a step that would let a failed run pass.
fn lane_runs(wf: &workflow::Workflow) -> Vec<String> {
    let job = wf.require_job("ci.yml", "docs-lane").unwrap();
    let mut out = Vec::new();
    if job.continue_on_error.is_some() {
        out.push("continue-on-error".to_string());
    }
    for step in &job.steps {
        if let Some(run) = &step.run {
            out.push(run.clone());
        }
        if step.continue_on_error.is_some() {
            out.push("continue-on-error".to_string());
        }
    }
    out
}

/// No required check's job has an `if:` that leaves it skipped on a pull
/// request: its own condition holds there, and a job that needs another
/// runs `always()`, so a skip upstream cannot carry into it.
#[test]
fn no_required_check_is_skipped_on_a_pull_request() {
    let root = workspace();
    for (check, id, file) in MAIN_REQUIRES {
        let text = std::fs::read_to_string(root.join(".github/workflows").join(file)).unwrap();
        let wf = workflow::Workflow::parse(&text).unwrap_or_else(|e| panic!("{file} parses: {e}"));
        let job = wf.require_job(file, id).unwrap();
        let own = job.if_cond.as_deref();
        let site = format!("{file}'s {id}");
        assert!(
            read_condition(own, &site).1.pull_request,
            "{check} skips on a pull request"
        );
        if !job.needs.is_empty() {
            assert!(
                own.is_some_and(|own| own.starts_with("always()")),
                "{check} needs a job whose skip it would carry"
            );
        }
    }
}
