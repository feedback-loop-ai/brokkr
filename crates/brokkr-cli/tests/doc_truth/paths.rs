//! The path rule: every repository path a code span outside a fence names
//! is in the tree.

use std::collections::BTreeSet;

use super::{doc, readable_docs, stale_allowances, Doc};
use crate::tracked_files;
use crate::workspace_root::workspace;

/// The code spans of a block's text, each with the byte it starts at, as
/// CommonMark reads them: a run of backticks opens a span the next run of
/// the same length closes, a run nothing closes is literal text, and so is
/// a backtick a backslash escapes outside a span: an odd run of them, since
/// two escape each other. The escape takes that one backtick, and the rest
/// of its run still opens a span.
fn code_spans(text: &str) -> Vec<(usize, &str)> {
    let run = |at: usize| text[at..].chars().take_while(|&c| c == '`').count();
    let escaped = |at: usize| text[..at].chars().rev().take_while(|&c| c == '\\').count() % 2 == 1;
    let mut spans = Vec::new();
    let mut at = 0;
    while let Some(start) = text[at..].find('`').map(|found| at + found) {
        if escaped(start) {
            at = start + 1;
            continue;
        }
        let ticks = run(start);
        let mut close = start + ticks;
        let closed = loop {
            match text[close..].find('`').map(|found| close + found) {
                Some(found) if run(found) == ticks => break Some(found),
                Some(found) => close = found + run(found),
                None => break None,
            }
        };
        match closed {
            Some(close) => {
                spans.push((start + ticks, text[start + ticks..close].trim()));
                at = close + ticks;
            }
            None => at = start + ticks,
        }
    }
    spans
}

/// The characters a repository path is written with here, a pattern's
/// placeholders included.
fn path_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || "._-/<>{}*…+@".contains(c)
}

/// The kind of file a path names, by which a name is read when the doc
/// leaves unsaid which directory holds it: what follows the last `.` of
/// its last segment when a name stands before it, so `README.md`,
/// `layering/scan.rs` and `social-preview.png` have one and `.gitignore`
/// and `CHANGELOG` none. [`Tree::kinds`] holds every kind the tree does.
fn kind(path: &str) -> Option<&str> {
    let (name, kind) = path.rsplit('/').next()?.rsplit_once('.')?;
    (!name.is_empty()).then_some(kind)
}

/// Where a path that opens with neither `./`, `../` nor a top-level
/// directory stands outside the repository, never read: a root a path goes
/// on beneath, so `<repo>/x.md` is foreign while `<run>.ndjson`, with no
/// `/`, is the name of a file the tree is asked for.
const FOREIGN: [(&str, &str); 8] = [
    ("/", "an absolute path names the host's tree"),
    ("~", "a path under a home directory"),
    ("$", "a path under an environment variable"),
    ("<", "a leading placeholder names a root the reader fills"),
    (".forge/", "a run's own working files, never tracked"),
    (
        ".brokkr/",
        "an adopter's Brokkr folder, which this repository, a recipe library, does not hold",
    ),
    ("target/", "cargo's build output"),
    ("node_modules/", "a Node repository's installed packages"),
];

/// Where a path a code span names is read from.
enum Named<'a> {
    /// From the root or the doc's directory, as [`resolved`] reads it: it
    /// opens with `./`, `../` or a top-level directory the tree tracks.
    Placed(&'a str),
    /// Under any directory: the name of a file of a [`kind`] the tree
    /// holds, or a path ending in one, whose start the doc leaves unsaid.
    Within(&'a str),
}

/// The repository path a code span names, if it names one: one word,
/// [`Named`] by how it opens, its `:line` or `#anchor` suffix dropped. A
/// word this cannot read as a path is an `Err`: a character no path here
/// carries, a suffix that is no line, or a placeholder nothing closes.
fn named_path<'a>(span: &'a str, tree: &Tree) -> Option<Result<Named<'a>, String>> {
    if span.contains(char::is_whitespace) {
        return None;
    }
    let path = span.split_once('#').map_or(span, |(path, _)| path);
    let (path, line) = path
        .split_once(':')
        .map_or((path, None), |(path, at)| (path, Some(at)));
    let named = match path.split_once('/') {
        Some((first, _)) if [".", ".."].contains(&first) || tree.tops.contains(first) => {
            Named::Placed(path)
        }
        Some(_) if FOREIGN.iter().any(|(root, _)| path.starts_with(root)) => return None,
        _ if kind(path).is_some_and(|kind| tree.kinds.contains(kind)) => Named::Within(path),
        _ => return None,
    };
    let lined =
        line.is_none_or(|at| !at.is_empty() && at.chars().all(|c| c.is_ascii_digit() || c == '-'));
    Some(if !lined {
        Err(format!("`{span}` reads as no path and line"))
    } else if let Some(c) = path.chars().find(|&c| !path_char(c)) {
        Err(format!("`{span}` carries `{c}`, which no path here does"))
    } else if let Some(open) = unclosed(path) {
        Err(format!(
            "`{span}` opens a placeholder `{open}` nothing closes"
        ))
    } else {
        Ok(named)
    })
}

/// The first placeholder opener (`<`, `{`) in `path` that its segment
/// never closes.
fn unclosed(path: &str) -> Option<char> {
    path.split('/').find_map(|segment| {
        let mut open = None;
        for c in segment.chars() {
            match (open, c) {
                (None, '<' | '{') => open = Some(c),
                (Some('<'), '>') | (Some('{'), '}') => open = None,
                _ => {}
            }
        }
        open
    })
}

/// A path pattern's segment with each placeholder (`<name>`, `{a,b}`,
/// `…`, `...`) written as `*`.
fn glob(segment: &str) -> Vec<char> {
    let mut out = Vec::new();
    let mut chars = segment.chars();
    while let Some(c) = chars.next() {
        let close = match c {
            '<' => '>',
            '{' => '}',
            '…' | '*' => {
                out.push('*');
                continue;
            }
            c => {
                out.push(c);
                continue;
            }
        };
        chars.by_ref().find(|&c| c == close);
        out.push('*');
    }
    let text: String = out.into_iter().collect();
    text.replace("...", "*").chars().collect()
}

/// Whether `name` fills a [`glob`]bed segment.
fn fills(pattern: &[char], name: &[char]) -> bool {
    match pattern.split_first() {
        None => name.is_empty(),
        Some(('*', rest)) => (0..=name.len()).any(|skip| fills(rest, &name[skip..])),
        Some((c, rest)) => name.first() == Some(c) && fills(rest, &name[1..]),
    }
}

/// The tracked tree: every tracked file and every directory above one,
/// the top-level directories, and the [`kind`] of every file.
struct Tree {
    entries: BTreeSet<String>,
    tops: BTreeSet<String>,
    kinds: BTreeSet<String>,
}

impl Tree {
    fn of(files: impl IntoIterator<Item = String>) -> Tree {
        let (mut entries, mut kinds) = (BTreeSet::new(), BTreeSet::new());
        for file in files {
            for (at, _) in file.match_indices('/') {
                entries.insert(file[..at].to_string());
            }
            kinds.extend(kind(&file).map(str::to_string));
            entries.insert(file);
        }
        let tops = entries
            .iter()
            .filter_map(|entry| entry.split_once('/').map(|(top, _)| top.to_string()))
            .collect();
        Tree {
            entries,
            tops,
            kinds,
        }
    }

    /// Whether the tree holds `path`, a pattern filled by any entry: from
    /// the root when `anchored`, else as the end of an entry's path.
    fn holds(&self, path: &str, anchored: bool) -> bool {
        let path = path.trim_end_matches('/');
        let wild = ['<', '{', '*', '…'].iter().any(|&c| path.contains(c)) || path.contains("...");
        if !wild && anchored {
            return self.entries.contains(path);
        }
        let pattern: Vec<Vec<char>> = path.split('/').map(glob).collect();
        self.entries.iter().any(|entry| {
            let names: Vec<&str> = entry.split('/').collect();
            let Some(start) = names.len().checked_sub(pattern.len()) else {
                return false;
            };
            (start == 0 || !anchored)
                && pattern
                    .iter()
                    .zip(&names[start..])
                    .all(|(segment, name)| fills(segment, &name.chars().collect::<Vec<_>>()))
        })
    }
}

/// Paths a living doc names that this repository rightly does not hold:
/// the doc (or the directory of docs, `""` for any), the path it names (or
/// the start of it), and why. An allowance that covers nothing fails, so
/// none outlives its text.
const ABSENT: [(&str, &str, &str); 26] = [
    (
        "",
        "credentials.toml",
        "a credential file the box masks in a home directory",
    ),
    (
        "",
        "hands.harness.work",
        "an adapter's bundle key, whose last word is a kind the tree holds",
    ),
    (
        "docs/guides/read-surfaces.md",
        ".jsonl.zstd",
        "the suffix of DSH's compressed session encoding, which names no file",
    ),
    (
        "docs/status.md",
        "session.v3.jsonl",
        "a dsh 0.1.5 session file under a seat's own root",
    ),
    (
        "extensions/dsh/PROVENANCE.md",
        "dsh.bundle.patch",
        "a DSH loader key, whose last word is a kind the tree holds",
    ),
    (
        "packaging/README.md",
        "Formula/brokkr.rb",
        "the formula in the sibling `homebrew-tap` repository",
    ),
    (
        "packaging/README.md",
        "tools/go.mod",
        "a module pin the page weighs and no file carries",
    ),
    (
        "",
        "CLAUDE.md",
        "Claude Code's memory file, the operator's own",
    ),
    (
        "",
        "tsconfig.json",
        "a Node repository's compiler configuration",
    ),
    (
        "",
        "*-tests.rs",
        "a test-file spelling the coverage gate reads, which no file here carries",
    ),
    (
        "",
        "alias/../",
        "a library spelling refused for climbing out of its directory",
    ),
    (
        "docs/research/github-peers/",
        "",
        "each page reads another project, whose files are its own",
    ),
    (
        "crates/brokkr-cli/dialects/speckit/",
        "specs/",
        "spec-kit's layout in the operator's repository",
    ),
    (
        "dialects/speckit/",
        "specs/",
        "spec-kit's layout in the operator's repository",
    ),
    (
        "docs/guides/contributing-by-hand.md",
        "coverage-",
        "the coverage gate's reports under `target/coverage/`",
    ),
    (
        "quality/README.md",
        "src/",
        "two files of an invented tree the example pairs",
    ),
    (
        "docs/guides/provider-adapters.md",
        "config.toml",
        "codex's own configuration",
    ),
    (
        "docs/guides/provider-adapters.md",
        "index.lock",
        "git's lock in a workspace's own `.git`",
    ),
    (
        "docs/guides/recipe-authoring.md",
        "../",
        "the spellings a recipe refuses, which never become paths",
    ),
    (
        "docs/guides/cards/bun.md",
        "recipes/bun",
        "says no such recipe ships",
    ),
    (
        "docs/guides/cards/go.md",
        "recipes/go",
        "says no such recipe ships",
    ),
    (
        "docs/guides/cards/python.md",
        "recipes/python",
        "says no such recipe ships",
    ),
    (
        "docs/guides/cards/rust.md",
        "recipes/rust",
        "says no such recipe ships",
    ),
    (
        "docs/guides/contributing-by-hand.md",
        ".cargo/config",
        "names the configuration the repository must not carry",
    ),
    (
        "extensions/dsh/PROVENANCE.md",
        "extensions/dsh/resume-policy/",
        "names the home an extension takes only once one is needed",
    ),
    (
        "recipes/wager-harness/README.md",
        "recipes/fast-codex",
        "names the recipe the wager deleted",
    ),
];

/// The paths from the root that `path`, named in a doc in the directory
/// `here` (`None` at the root), may be: a path that climbs with `../` is
/// resolved against `here`, and any other is read from the root and from
/// `here`, a leading `./` dropped. A path that climbs above the root is an
/// `Err`.
fn resolved(path: &str, here: Option<&str>) -> Result<Vec<String>, String> {
    let mut dir: Vec<&str> = here.map_or_else(Vec::new, |here| here.split('/').collect());
    let mut rest = path;
    let mut climbed = false;
    loop {
        if let Some(after) = rest.strip_prefix("./") {
            rest = after;
        } else if let Some(after) = rest.strip_prefix("../") {
            dir.pop()
                .ok_or_else(|| format!("`{path}` climbs above the repository"))?;
            (rest, climbed) = (after, true);
        } else {
            break;
        }
    }
    let within = |dir: &[&str]| match dir {
        [] => rest.to_string(),
        dir => format!("{}/{rest}", dir.join("/")),
    };
    Ok(match climbed {
        true => vec![within(&dir)],
        false => vec![rest.to_string(), within(&dir)],
    })
}

/// Why a code span in a doc in the directory `here` is false: it names a
/// path the tree does not hold, read as its [`Named`] says, or it cannot
/// be read. `None` when it names no path or a path the tree holds.
fn unheld(span: &str, tree: &Tree, here: Option<&str>) -> Option<String> {
    let held = named_path(span, tree)?.and_then(|named| match named {
        Named::Placed(path) => {
            resolved(path, here).map(|paths| paths.iter().any(|path| tree.holds(path, true)))
        }
        Named::Within(path) => Ok(tree.holds(path, false)),
    });
    match held {
        Ok(true) => None,
        Ok(false) => Some(format!("`{span}` names a path the tree does not hold")),
        Err(why) => Some(why),
    }
}

/// Each code span in `docs` that names a path the tree does not hold, and
/// each span that cannot be read, outside what `absent`, given [`ABSENT`],
/// allows by the span's start; and each allowance that covered nothing.
fn missing_paths(docs: &[Doc], tree: &Tree, absent: &[(&str, &str, &str)]) -> Vec<String> {
    let mut found = Vec::new();
    let mut used = vec![false; absent.len()];
    for doc in docs {
        let here = doc.path.rsplit_once('/').map(|(dir, _)| dir);
        for block in doc.blocks.iter().filter(|block| !block.fenced) {
            for (at, span) in code_spans(&block.text) {
                let Some(why) = unheld(span, tree, here) else {
                    continue;
                };
                match absent.iter().position(|(page, named, _)| {
                    doc.path.starts_with(page) && span.starts_with(named)
                }) {
                    Some(index) => used[index] = true,
                    None => found.push(format!("{}:{}: {why}", doc.path, block.line_of(at))),
                }
            }
        }
    }
    found.extend(stale_allowances(absent, &used));
    found
}

#[test]
fn every_path_a_living_doc_names_is_in_the_tree() {
    let tree = Tree::of(tracked_files::tracked(&workspace(), &[]));
    assert_eq!(
        missing_paths(&readable_docs(), &tree, &ABSENT),
        Vec::<String>::new(),
        "a living doc names a repository path that is not there"
    );
}

#[test]
fn a_path_the_tree_does_not_hold_is_refused() {
    let tree = Tree::of(
        [
            "docs/guides/quickstart.md",
            "recipes/fast/bundle.json",
            "recipes/fast/scripts/verify-seat.sh",
            "scripts/coverage-exact.sh",
            "crates/brokkr-cli/src/main.rs",
        ]
        .map(str::to_string),
    );
    let planted = doc(
        "recipes/fast/README.md",
        "Read `docs/guides/quickstart.md#step-1`, `scripts/coverage-exact.sh:12-20`\n\
         and `recipes/<name>/bundle.json`, run `scripts/verify-seat.sh` here,\n\
         and keep `docs/guides/` and `recipes/*/scripts/`.\n\
         \n\
         Not `docs/target-architecture.md`, nor `recipes/<name>/policy.json`.\n\
         \n\
         `docs/a b.md` and `bundles/self` and `.forge/x` are no repository path.\n\
         \n\
         Unreadable: `docs/guides/quick$start.md`, `scripts/x.sh:main`.\n\
         \n\
         ```\n\
         `docs/never.md` in a fence is a sample\n\
         ```\n\
         \n\
         Escaped \\`docs/y.md\\`, then `docs/b.md`, then one left open: `docs/x.md\n\
         \n\
         There is no `recipes/bun`.\n\
         \n\
         Relative: `./recipes/fast/bundle.json`, `./bundle.json`, `../fast/bundle.json`,\n\
         `../../scripts/coverage-exact.sh`, `./gone.md`, `../../scripts/gone.sh`,\n\
         `../bundle.json`, `../../../x.md`, `./bad[1].sh`.\n\
         \n\
         Unsaid: `quickstart.md`, `guides/quickstart.md`, `bundle.json:3`, `doc/guides/quickstart.md`,\n\
         `README-missing.md`, `missing.rs:main`, `owner/model`, `CHANGELOG`.\n\
         Unclosed: `docs/<guide/quickstart.md`, `recipes/{name/bundle.json`, `recipes/<name>/{x}.json`.\n\
         Foreign: `~/.cargo/credentials.toml`, `$HOME/x.md`, `/usr/x.sh`, `<repo>/x.md`,\n\
         `.forge/x.json`, `target/x.json`, `node_modules/x.json`.\n",
    )
    .unwrap();
    let root = doc("README.md", "Not `scripts/verify-seat.sh` from the root.\n").unwrap();
    let absent = [
        ("recipes/fast/", "recipes/bun", "says no such recipe ships"),
        (
            "recipes/fast/README.md",
            "recipes/go",
            "says no such recipe ships",
        ),
        ("recipes/fast/", "./bad[", "a spelling refused on purpose"),
    ];
    assert_eq!(
        missing_paths(&[planted, root], &tree, &absent),
        [
            "recipes/fast/README.md:5: `docs/target-architecture.md` names a path the tree does not hold",
            "recipes/fast/README.md:5: `recipes/<name>/policy.json` names a path the tree does not hold",
            "recipes/fast/README.md:9: `docs/guides/quick$start.md` carries `$`, which no path here does",
            "recipes/fast/README.md:9: `scripts/x.sh:main` reads as no path and line",
            "recipes/fast/README.md:15: `docs/b.md` names a path the tree does not hold",
            "recipes/fast/README.md:20: `./gone.md` names a path the tree does not hold",
            "recipes/fast/README.md:20: `../../scripts/gone.sh` names a path the tree does not hold",
            "recipes/fast/README.md:21: `../bundle.json` names a path the tree does not hold",
            "recipes/fast/README.md:21: `../../../x.md` climbs above the repository",
            "recipes/fast/README.md:23: `doc/guides/quickstart.md` names a path the tree does not hold",
            "recipes/fast/README.md:24: `README-missing.md` names a path the tree does not hold",
            "recipes/fast/README.md:24: `missing.rs:main` reads as no path and line",
            "recipes/fast/README.md:25: `docs/<guide/quickstart.md` opens a placeholder `<` nothing closes",
            "recipes/fast/README.md:25: `recipes/{name/bundle.json` opens a placeholder `{` nothing closes",
            "README.md:1: `scripts/verify-seat.sh` names a path the tree does not hold",
            "recipes/fast/README.md: allows `recipes/go`, which it no longer names",
        ]
    );
    let escapes = doc(
        "docs/guides/b.md",
        r"Two escape each other: \\`docs/two.md`; three escape it: \\\`docs/three.md`.",
    )
    .unwrap();
    let run = doc(
        "docs/guides/c.md",
        r"An escape takes one: \``docs/one.md`; of three: \```docs/run.md``.",
    )
    .unwrap();
    assert_eq!(
        missing_paths(&[escapes, run], &tree, &[]),
        [
            "docs/guides/b.md:1: `docs/two.md` names a path the tree does not hold",
            "docs/guides/c.md:1: `docs/one.md` names a path the tree does not hold",
            "docs/guides/c.md:1: `docs/run.md` names a path the tree does not hold",
        ]
    );
}

#[test]
fn a_name_of_any_kind_the_tree_holds_is_read_as_a_path() {
    let tree = Tree::of(
        [
            "docs/assets/logo.svg",
            ".github/social-preview.png",
            "crates/brokkr-cli/src/ui.html",
            "docs/evidence/r1.ndjson",
        ]
        .map(str::to_string),
    );
    let planted = doc(
        "docs/a.md",
        "Held: `assets/logo.svg`, `social-preview.png`, `ui.html`, `src/ui.html`, `<run>.ndjson`.\n\
         \n\
         Not held: `asset/logo.svg`, `social.png`, `web/ui.html:3`, `<run>.trace.ndjson`.\n\
         \n\
         No kind the tree holds: `logo.webp`, `<run>.ndjsn`, `.gitignore`, `LICENSE`.\n\
         Foreign: `<repo>/r1.ndjson`, `<repo>/gone.ndjson`.\n",
    )
    .unwrap();
    let refused = [
        "asset/logo.svg",
        "social.png",
        "web/ui.html:3",
        "<run>.trace.ndjson",
    ]
    .map(|span| format!("docs/a.md:3: `{span}` names a path the tree does not hold"));
    assert_eq!(missing_paths(&[planted], &tree, &[]), refused);
}
