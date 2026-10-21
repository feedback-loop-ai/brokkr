//! The version rule: every version of Brokkr a doc states is the
//! workspace's.

use super::{doc, readable_docs, Doc};
use crate::packaging;

/// The comment that marks the next block's Brokkr versions illustrative: a
/// sample whose version is beside its point.
const ILLUSTRATIVE: &str = "<!-- illustrative version -->";

/// A spelling that states Brokkr's own version: the text before the
/// version, whether a [`gap`] parts the two, the characters that open it,
/// each after any whitespace, and the text that closes it.
struct Spelling {
    before: &'static str,
    gapped: bool,
    open: &'static str,
    close: &'static str,
}

/// A word a version follows past a [`gap`]: `brokkr 0.12.0`, `Brokkr
/// **0.12.0**`, `Brokkr version: 0.12.0`, `Brokkr release 0.12.0`, a
/// table's `| Brokkr | 0.12.0 |`, the word ending one line and the version
/// opening the next.
const fn word(before: &'static str) -> Spelling {
    Spelling {
        before,
        gapped: true,
        open: "",
        close: "",
    }
}

/// The text after the gap that parts a word from the version it names:
/// whitespace, line breaks included, inline markup (`*`, `_`, `` ` ``),
/// label punctuation (`:`, a table cell's `|`) and a `version`,
/// `versions`, `release`, `releases` or `released` word. `None` when
/// nothing parts them.
fn gap(text: &str) -> Option<&str> {
    let parts = |c: char| c.is_whitespace() || "*_`:|".contains(c);
    let rest = text.trim_start_matches(parts);
    if rest.len() == text.len() {
        return None;
    }
    let label = ["version", "versions", "release", "releases", "released"]
        .into_iter()
        .find_map(|label| {
            rest.strip_prefix(label)
                .filter(|after| after.starts_with(parts))
        });
    Some(label.map_or(rest, |after| after.trim_start_matches(parts)))
}

/// The spellings that state Brokkr's own version. One with an opener is
/// read once the opener is; one without is read when a number follows its
/// text and, when it has a closer, its closer follows that word. A spelling
/// read but followed by no version this can read is refused.
const VERSION_SPELLINGS: [Spelling; 5] = [
    word("brokkr"),
    word("Brokkr"),
    word("engine"),
    // `"engine_version": "0.12.0"` and serde_json's compact form alike.
    Spelling {
        before: "\"engine_version\"",
        gapped: false,
        open: ":\"",
        close: "\"",
    },
    // A link that headlines a release: "The [v0.12.0 release](…)".
    Spelling {
        before: "[v",
        gapped: false,
        open: "",
        close: " release",
    },
];

/// The text after `open` at the start of `text`, any whitespace before
/// each of its characters skipped.
fn opened<'a>(text: &'a str, open: &str) -> Option<&'a str> {
    open.chars()
        .try_fold(text, |rest, c| rest.trim_start().strip_prefix(c))
}

/// The version `text` opens with (`0.12.0`, `v0.12.0`, `0.1.5-rc.1`,
/// `0.12.0+build.123`), its pre-release and build metadata included,
/// without its `v`, and the text after it.
fn version(text: &str) -> Option<(&str, &str)> {
    let bare = text.strip_prefix('v').unwrap_or(text);
    let mut parts = 0;
    let mut end = 0;
    for part in bare.split('.') {
        let digits = part.chars().take_while(char::is_ascii_digit).count();
        if digits == 0 || parts == 3 {
            break;
        }
        parts += 1;
        end += digits;
        if digits < part.len() || parts == 3 {
            break;
        }
        end += 1;
    }
    if parts < 3 {
        return None;
    }
    let label = |at: usize, mark: char| {
        bare[at..].strip_prefix(mark).map_or(0, |label| {
            let label = label
                .split(|c: char| !c.is_ascii_alphanumeric() && c != '.')
                .next()
                .unwrap_or_default();
            1 + label.trim_end_matches('.').len()
        })
    };
    let end = end + label(end, '-');
    let end = end + label(end, '+');
    let rest = &bare[end..];
    let joined = rest.starts_with(|c: char| c.is_ascii_alphanumeric())
        || rest
            .strip_prefix('.')
            .is_some_and(|after| after.starts_with(|c: char| c.is_ascii_digit()));
    (!joined).then(|| (&bare[..end], rest))
}

/// Each version of Brokkr `text` states by a [`VERSION_SPELLINGS`] entry,
/// with the byte it stands at: the version, or the spelling that no
/// version this can read follows.
fn versions_in(text: &str) -> Vec<(usize, Result<&str, &'static str>)> {
    let mut found = Vec::new();
    for Spelling {
        before,
        gapped,
        open,
        close,
    } in VERSION_SPELLINGS
    {
        for (at, _) in text.match_indices(before) {
            if text[..at].ends_with(|c: char| c.is_alphanumeric() || c == '-' || c == '_') {
                continue;
            }
            let after = &text[at + before.len()..];
            let Some(rest) = (match gapped {
                true => gap(after),
                false => Some(after),
            })
            .and_then(|after| opened(after, open)) else {
                continue;
            };
            let numbered = rest
                .trim_start_matches('v')
                .starts_with(|c: char| c.is_ascii_digit());
            let word = rest.find(char::is_whitespace).unwrap_or(rest.len());
            let read = !open.is_empty() || (numbered && rest[word..].starts_with(close));
            match version(rest) {
                Some((stated, tail)) if tail.starts_with(close) => found.push((at, Ok(stated))),
                _ if read => found.push((at, Err(before))),
                _ => {}
            }
        }
    }
    found
}

/// Each Brokkr version in `docs` that is not `current`, outside a block
/// marked [`ILLUSTRATIVE`], and each version word this cannot read.
fn stale_versions(docs: &[Doc], current: &str) -> Vec<String> {
    let mut found = Vec::new();
    for doc in docs {
        let mut marked = false;
        for block in &doc.blocks {
            let illustrative = std::mem::replace(&mut marked, block.text.trim() == ILLUSTRATIVE);
            for (at, stated) in versions_in(&block.text) {
                let line = block.line_of(at);
                match stated {
                    Ok(stated) if stated == current || illustrative => {}
                    Ok(stated) => found.push(format!(
                        "{}:{line}: states Brokkr {stated}; the workspace is {current}",
                        doc.path
                    )),
                    Err(before) => found.push(format!(
                        "{}:{line}: `{before}` is followed by no version this can read",
                        doc.path
                    )),
                }
            }
        }
    }
    found
}

#[test]
fn every_brokkr_version_a_living_doc_states_is_the_workspaces() {
    assert_eq!(
        stale_versions(&readable_docs(), &packaging::workspace_version()),
        Vec::<String>::new(),
        "a living doc states a version of Brokkr that is not Cargo.toml's: \
         write the workspace's, or mark the sample {ILLUSTRATIVE}"
    );
}

#[test]
fn a_stale_version_is_refused_unless_marked_illustrative() {
    let planted = doc(
        "docs/guides/a.md",
        "The [v0.11.0 release](../releases/v0.11.0.md) adds things;\n\
         the [v0.10.0 notes](../releases/v0.10.0.md) say more.\n\
         \n\
         ```\n\
         $ brokkr --version\n\
         brokkr 0.8.0\n\
         ok contracts: engine 0.12.0, event_schema 1\n\
         ```\n\
         \n\
         <!-- illustrative version -->\n\
         \n\
         ```\n\
         { \"type\": \"hello\", \"engine_version\": \"0.5.0\" }\n\
         ```\n\
         \n\
         { \"type\": \"hello\", \"engine_version\": \"0.7.0\" }\n\
         \n\
         Brokkr v0.12.0 ships; the dsh-engine 0.1.5 differs; brokkr init runs.\n\
         \n\
         brokkr 0.12 and engine 13/13 read as no version.\n\
         \n\
         {\"engine_version\":\"0.8.0\"}, {\"engine_version\" : \"0.12\"}, the [v0.12 release](x).\n\
         \n\
         Released as Brokkr\n\
         0.11.0 last week; brokkr  0.10.0, brokkr\t0.9.0,\n\
         Brokkr **0.8.1** and Brokkr version 0.8.2; `brokkr_core 0.1.0` and brokkr-cli 0.1.0 are none.\n",
    )
    .unwrap();
    assert_eq!(
        stale_versions(&[planted], "0.12.0"),
        [
            "docs/guides/a.md:1: states Brokkr 0.11.0; the workspace is 0.12.0",
            "docs/guides/a.md:6: states Brokkr 0.8.0; the workspace is 0.12.0",
            "docs/guides/a.md:16: states Brokkr 0.7.0; the workspace is 0.12.0",
            "docs/guides/a.md:20: `brokkr` is followed by no version this can read",
            "docs/guides/a.md:20: `engine` is followed by no version this can read",
            "docs/guides/a.md:22: states Brokkr 0.8.0; the workspace is 0.12.0",
            "docs/guides/a.md:22: `\"engine_version\"` is followed by no version this can read",
            "docs/guides/a.md:22: `[v` is followed by no version this can read",
            "docs/guides/a.md:25: states Brokkr 0.10.0; the workspace is 0.12.0",
            "docs/guides/a.md:25: states Brokkr 0.9.0; the workspace is 0.12.0",
            "docs/guides/a.md:24: states Brokkr 0.11.0; the workspace is 0.12.0",
            "docs/guides/a.md:26: states Brokkr 0.8.1; the workspace is 0.12.0",
            "docs/guides/a.md:26: states Brokkr 0.8.2; the workspace is 0.12.0",
        ]
    );
    let quoted = doc(
        "docs/guides/q.md",
        "> Released as Brokkr\n> 0.11.0 last week.\n",
    )
    .unwrap();
    assert_eq!(
        stale_versions(&[quoted], "0.12.0"),
        ["docs/guides/q.md:1: states Brokkr 0.11.0; the workspace is 0.12.0"]
    );
    let labelled = doc(
        "docs/guides/l.md",
        "Brokkr: 0.11.0, Brokkr version: 0.10.0 and Brokkr release 0.9.0.\n\
         \n\
         | Tool | Version |\n\
         | --- | --- |\n\
         | Brokkr | 0.8.0 |\n\
         \n\
         Brokkr: 0.12, Brokkr release notes, brokkr::cli and Brokkr v0.7.0.\n",
    )
    .unwrap();
    assert_eq!(
        stale_versions(&[labelled], "0.12.0"),
        [
            "docs/guides/l.md:1: states Brokkr 0.11.0; the workspace is 0.12.0",
            "docs/guides/l.md:1: states Brokkr 0.10.0; the workspace is 0.12.0",
            "docs/guides/l.md:1: states Brokkr 0.9.0; the workspace is 0.12.0",
            "docs/guides/l.md:5: states Brokkr 0.8.0; the workspace is 0.12.0",
            "docs/guides/l.md:7: `Brokkr` is followed by no version this can read",
            "docs/guides/l.md:7: states Brokkr 0.7.0; the workspace is 0.12.0",
        ]
    );
    assert_eq!(version("0.1.5-rc.1. More"), Some(("0.1.5-rc.1", ". More")));
    assert_eq!(version("v0.12.0 release"), Some(("0.12.0", " release")));
    assert_eq!(version("0.12.0.1"), None);
}

#[test]
fn build_metadata_and_every_release_word_are_read_with_the_version() {
    let planted = doc(
        "docs/guides/m.md",
        "Brokkr 0.12.0+build.123 and {\"engine_version\": \"0.12.0+build.123\"} are built.\n\
         \n\
         Brokkr releases 0.11.0, Brokkr released 0.10.0 and Brokkr versions 0.9.0.\n\
         \n\
         Brokkr releases are signed, Brokkr versions differ and Brokkr 0.12.0 ships.\n",
    )
    .unwrap();
    let states = |(line, stated): (usize, &str)| {
        format!("docs/guides/m.md:{line}: states Brokkr {stated}; the workspace is 0.12.0")
    };
    let built = "0.12.0+build.123";
    assert_eq!(
        stale_versions(&[planted], "0.12.0"),
        [
            (1, built),
            (1, built),
            (3, "0.11.0"),
            (3, "0.10.0"),
            (3, "0.9.0")
        ]
        .map(states)
    );
    assert_eq!(
        version("0.1.5-rc.1+b2. More"),
        Some(("0.1.5-rc.1+b2", ". More"))
    );
}
