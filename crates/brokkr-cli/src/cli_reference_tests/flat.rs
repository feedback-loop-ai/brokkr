//! No old flat spelling comes back to the guides, the front page or the
//! reference pages before the aliases go in 0.17 (decision 0113 ruling 6,
//! #1169). A `brokkr <root>` whose root moved under a family is judged by
//! `respelled` itself, the one home of the moves, so a root that moves
//! later is refused here with no second list to keep. Prose that names a
//! verb without `brokkr` before it, and the records outside these pages,
//! are not read.

use std::path::Path;

use super::{tracked, workspace};

/// The one line a flat spelling stands on purpose: the reference's own
/// account of the aliases, which names an old spelling beside its new one.
const ALIASES_NAMED: (&str, &str) = ("docs/reference/cli.md", "Every old flat name, such as");

/// The pages held to decision 0113's tree: the guides, the front page and
/// the reference pages, as git tracks them.
fn held_pages(root: &Path) -> Vec<String> {
    tracked::tracked(root, &["*.md"])
        .into_iter()
        .filter(|path| {
            path == "README.md"
                || path.starts_with("docs/guides/")
                || path.starts_with("docs/reference/")
        })
        .collect()
}

/// `word` cut to the letters, digits and hyphens a verb is spelled in.
fn verb(word: &str) -> String {
    word.chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect()
}

/// The first two words after `brokkr` at `after`, across any line break.
fn next_words(after: &str) -> [String; 2] {
    let mut words = after.split_whitespace().map(verb);
    [
        words.next().unwrap_or_default(),
        words.next().unwrap_or_default(),
    ]
}

/// The first two arguments after a launchd plist's `<string>…brokkr</string>`,
/// each its own `<string>` element, across any line break. `None` when
/// `after` does not close the program's element.
fn plist_words(after: &str) -> Option<[String; 2]> {
    let mut rest = after.strip_prefix("</string>")?;
    let mut words = [String::new(), String::new()];
    for word in &mut words {
        let Some(open) = rest.trim_start().strip_prefix("<string>") else {
            break;
        };
        let Some((text, tail)) = open.split_once("</string>") else {
            break;
        };
        *word = verb(text);
        rest = tail;
    }
    Some(words)
}

/// Each `brokkr <root>` in `doc` that the binary respells, as
/// `file:line: <the alias's own note>`. `brokkr` counts as the program
/// word bare or at the end of a path (`./brokkr`, `target/release/brokkr`,
/// `%h/.local/bin/brokkr`), followed by a space or a line break, or as a
/// plist's `<string>…brokkr</string>` with its arguments in the elements
/// after it. So `brokkr-cli`, `.brokkr agents` and `~/src/brokkr/recipes`
/// are never read as one.
fn flat_spellings(file: &str, doc: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (at, word) in doc.match_indices("brokkr") {
        let before = doc[..at].chars().next_back();
        let after = &doc[at + word.len()..];
        if before.is_some_and(|c| c.is_alphanumeric() || "_-.".contains(c)) {
            continue;
        }
        let [first, second] = if after.starts_with(char::is_whitespace) {
            next_words(after)
        } else if let Some(words) = plist_words(after) {
            words
        } else {
            continue;
        };
        let Some(moved) = crate::cli_args::respelled(["brokkr", first.as_str(), second.as_str()]).1
        else {
            continue;
        };
        let line = doc[..at].matches('\n').count() + 1;
        let text = doc.lines().nth(line - 1).unwrap_or_default();
        if file == ALIASES_NAMED.0 && text.contains(ALIASES_NAMED.1) {
            continue;
        }
        found.push(format!("{file}:{line}: {moved}"));
    }
    found
}

#[test]
fn no_guide_front_page_or_reference_page_types_a_flat_spelling() {
    let root = workspace();
    let pages = held_pages(&root);
    assert!(pages.iter().any(|page| page == "docs/guides/quickstart.md"));
    assert!(pages.iter().any(|page| page == ALIASES_NAMED.0));
    let mut found = Vec::new();
    for page in &pages {
        let text = std::fs::read_to_string(root.join(page)).expect("a tracked page reads");
        found.extend(flat_spellings(page, &text));
    }
    assert_eq!(
        found.join("\n"),
        "",
        "flat spellings of moved roots (decision 0113)"
    );
}

#[test]
fn a_flat_spelling_is_named_wherever_brokkr_types_it_and_nowhere_else() {
    let page = "$ brokkr inspect --run latest\n\
                run `brokkr\nexport` then brokkr help seats\n\
                brokkr runs inspect, brokkr-cli compile, ~/src/brokkr/recipes, \
                .brokkr agents, brokkr resume\n\
                Every old flat name, such as `brokkr verify-run`, parses\n\
                ./brokkr inspect, target/release/brokkr queue dispatch --once\n\
                ExecStart=%h/.local/bin/brokkr dispatch --every 60\n\
                <string>/usr/local/bin/brokkr</string>\n  <string>compile</string>\n\
                <string>brokkr</string><string>queue</string><string>dispatch</string>\n";
    assert_eq!(
        flat_spellings("docs/guides/a.md", page),
        [
            "docs/guides/a.md:1: brokkr: 'inspect' is now 'brokkr runs inspect'; \
             the old spelling goes in 0.17",
            "docs/guides/a.md:2: brokkr: 'export' is now 'brokkr journal export'; \
             the old spelling goes in 0.17",
            "docs/guides/a.md:3: brokkr: 'seats' is now 'brokkr runs seats'; \
             the old spelling goes in 0.17",
            "docs/guides/a.md:5: brokkr: 'verify-run' is now 'brokkr journal verify'; \
             the old spelling goes in 0.17",
            "docs/guides/a.md:6: brokkr: 'inspect' is now 'brokkr runs inspect'; \
             the old spelling goes in 0.17",
            "docs/guides/a.md:7: brokkr: 'dispatch' is now 'brokkr queue dispatch'; \
             the old spelling goes in 0.17",
            "docs/guides/a.md:8: brokkr: 'compile' is now 'brokkr library compile'; \
             the old spelling goes in 0.17",
        ]
    );
    assert_eq!(
        flat_spellings(ALIASES_NAMED.0, page),
        [
            "docs/reference/cli.md:1: brokkr: 'inspect' is now 'brokkr runs inspect'; \
             the old spelling goes in 0.17",
            "docs/reference/cli.md:2: brokkr: 'export' is now 'brokkr journal export'; \
             the old spelling goes in 0.17",
            "docs/reference/cli.md:3: brokkr: 'seats' is now 'brokkr runs seats'; \
             the old spelling goes in 0.17",
            "docs/reference/cli.md:6: brokkr: 'inspect' is now 'brokkr runs inspect'; \
             the old spelling goes in 0.17",
            "docs/reference/cli.md:7: brokkr: 'dispatch' is now 'brokkr queue dispatch'; \
             the old spelling goes in 0.17",
            "docs/reference/cli.md:8: brokkr: 'compile' is now 'brokkr library compile'; \
             the old spelling goes in 0.17",
        ]
    );
    // `brokkr` opening the page is read; one glued by a letter or `_` is
    // not; a plist element with no argument after it, or one left open,
    // names nothing.
    let edges = "brokkr export --run r\n\
                 xbrokkr inspect, my_brokkr inspect\n\
                 <string>brokkr</string>\n</array>\n\
                 <string>brokkr</string><string>compile";
    assert_eq!(
        flat_spellings("docs/guides/b.md", edges),
        [
            "docs/guides/b.md:1: brokkr: 'export' is now 'brokkr journal export'; \
             the old spelling goes in 0.17"
        ]
    );
}
