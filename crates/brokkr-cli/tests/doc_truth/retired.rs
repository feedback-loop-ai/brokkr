//! The retired-term rule: no retired term stands in a block that does not
//! cite what retired it.

use super::{doc, readable_docs, stale_allowances, Doc};

/// A term the house retired, by each spelling it is written in, why, and
/// the citation that lets a block name it: a block that cites what retired
/// it says so, and is read true.
struct Retired {
    spellings: &'static [&'static str],
    why: &'static str,
    cite: Option<&'static str>,
}

/// The decision that retired the third host.
const NO_WINDOWS: &str = "decision 0063: the hosts are Linux and macOS";

/// Never built, as `docs/history/target-architecture.md`'s status records.
const NEVER_BUILT: &str = "never built (#364): the DeepSeek harness is the built-in dsh adapter";

const RETIRED: [Retired; 8] = [
    // Lowercase `windows` alone is a context window; opening a hyphenated
    // word it is a runner label (`windows-latest`, `windows-2022`).
    Retired {
        spellings: &["Windows", "WINDOWS", "windows-"],
        why: NO_WINDOWS,
        cite: Some("0063"),
    },
    Retired {
        spellings: &["winget", "Winget", "WinGet", "WINGET"],
        why: NO_WINDOWS,
        cite: Some("0063"),
    },
    Retired {
        spellings: &["scoop", "Scoop"],
        why: NO_WINDOWS,
        cite: Some("0063"),
    },
    // The Cordis/DeepSeek harness; Cordis stays dsh's own framework, which
    // [`ALLOWED`] lets the dsh extension name.
    Retired {
        spellings: &["Cordis"],
        why: NEVER_BUILT,
        cite: None,
    },
    Retired {
        spellings: &["Surface"],
        why: NEVER_BUILT,
        cite: None,
    },
    Retired {
        spellings: &["brokkr-api"],
        why: "never built (#364): `brokkr ui` serves the console from brokkr-cli",
        cite: None,
    },
    Retired {
        spellings: &["policy/schemas"],
        why: "removed: `policy/` holds the production table alone",
        cite: None,
    },
    Retired {
        spellings: &["bundles/"],
        why: "decision 0078 moved Brokkr's own bundles into `recipes/` (#359)",
        cite: None,
    },
];

/// Living docs that may name a retired term, each with its reason. An
/// allowance that covers nothing fails, so none outlives its text.
const ALLOWED: [(&str, &str, &str); 2] = [
    (
        "extensions/dsh/",
        "Cordis",
        "dsh's own framework, which the vendored extension builds on",
    ),
    (
        "docs/research/",
        "Windows",
        "a research entry reads another project, whose hosts are its own",
    ),
];

/// Whether `term` stands at `at` of `text` as a word of its own: no word
/// runs on into either end of it that is a word's (`bundles/` may open a
/// longer path).
fn whole_word(text: &str, at: usize, term: &str) -> bool {
    let word = |c: char| c.is_alphanumeric() || c == '_';
    let open = term.starts_with(word);
    let close = term.ends_with(word);
    (!open || text[..at].chars().next_back().is_none_or(|c| !word(c)))
        && !text[..at].ends_with('-')
        && (!close
            || text[at + term.len()..]
                .chars()
                .next()
                .is_none_or(|c| !word(c)))
}

/// Each retired term a block's `text` names as a word of its own, unless
/// the block cites what retired it: the byte it stands at, the term, and
/// why it was retired.
fn retired_in(text: &str) -> Vec<(usize, &'static str, &'static str)> {
    let mut found = Vec::new();
    for Retired {
        spellings,
        why,
        cite,
    } in RETIRED
    {
        if cite.is_some_and(|cite| text.contains(cite)) {
            continue;
        }
        for &term in spellings {
            found.extend(
                text.match_indices(term)
                    .filter(|&(at, _)| whole_word(text, at, term))
                    .map(|(at, _)| (at, term, why)),
            );
        }
    }
    found
}

/// Each retired term in `docs` outside a block that cites what retired it,
/// and each [`ALLOWED`] entry, given as `allowed`, that covered nothing.
fn retired_terms(docs: &[Doc], allowed: &[(&str, &str, &str)]) -> Vec<String> {
    let mut found = Vec::new();
    let mut used = vec![false; allowed.len()];
    let blocks = docs
        .iter()
        .flat_map(|doc| doc.blocks.iter().map(move |block| (doc, block)));
    for (doc, block) in blocks {
        for (at, term, why) in retired_in(&block.text) {
            let allowance = allowed
                .iter()
                .position(|(path, allowed, _)| *allowed == term && doc.path.starts_with(path));
            match allowance {
                Some(index) => used[index] = true,
                None => found.push(format!(
                    "{}:{}: names `{term}`, retired: {why}",
                    doc.path,
                    block.line_of(at)
                )),
            }
        }
    }
    found.extend(stale_allowances(allowed, &used));
    found
}

#[test]
fn no_living_doc_names_a_retired_term() {
    assert_eq!(
        retired_terms(&readable_docs(), &ALLOWED),
        Vec::<String>::new(),
        "a living doc names a retired term: say what replaced it, or cite what retired it"
    );
}

#[test]
fn a_retired_term_is_refused_outside_its_citation_and_its_allowance() {
    let planted = doc(
        "docs/guides/a.md",
        "Windows is not a host (decision 0063).\n\
         \n\
         Install with winget or scoop on Windows.\n\
         \n\
         The `brokkr-api` crate, a Surface driver, the Cordis/DeepSeek harness,\n\
         `policy/schemas/` and `bundles/self` are gone; `<repo>/bundles/fast` too.\n\
         \n\
         Context windows, a read surface, recipes/my-bundles/ and cordis.patch.yml are not.\n\
         \n\
         WINDOWS hosts, Winget, WinGet, WINGET, Scoop and a Cordis hook.\n\
         \n\
         runs-on: windows-latest; not context windows, context-windows-wide or 356-windows-is.\n",
    )
    .unwrap();
    let research = doc("docs/research/a.md", "CI runs on Windows.\n").unwrap();
    let allowed = [
        ("docs/research/", "Windows", "another project's hosts"),
        ("docs/guides/b.md", "winget", "stale"),
    ];
    assert_eq!(
        retired_terms(&[planted, research], &allowed),
        [
            "docs/guides/a.md:3: names `Windows`, retired: decision 0063: the hosts are Linux and macOS",
            "docs/guides/a.md:3: names `winget`, retired: decision 0063: the hosts are Linux and macOS",
            "docs/guides/a.md:3: names `scoop`, retired: decision 0063: the hosts are Linux and macOS",
            "docs/guides/a.md:5: names `Cordis`, retired: never built (#364): the DeepSeek harness is the built-in dsh adapter",
            "docs/guides/a.md:5: names `Surface`, retired: never built (#364): the DeepSeek harness is the built-in dsh adapter",
            "docs/guides/a.md:5: names `brokkr-api`, retired: never built (#364): `brokkr ui` serves the console from brokkr-cli",
            "docs/guides/a.md:6: names `policy/schemas`, retired: removed: `policy/` holds the production table alone",
            "docs/guides/a.md:6: names `bundles/`, retired: decision 0078 moved Brokkr's own bundles into `recipes/` (#359)",
            "docs/guides/a.md:6: names `bundles/`, retired: decision 0078 moved Brokkr's own bundles into `recipes/` (#359)",
            "docs/guides/a.md:10: names `WINDOWS`, retired: decision 0063: the hosts are Linux and macOS",
            "docs/guides/a.md:10: names `Winget`, retired: decision 0063: the hosts are Linux and macOS",
            "docs/guides/a.md:10: names `WinGet`, retired: decision 0063: the hosts are Linux and macOS",
            "docs/guides/a.md:10: names `WINGET`, retired: decision 0063: the hosts are Linux and macOS",
            "docs/guides/a.md:10: names `Scoop`, retired: decision 0063: the hosts are Linux and macOS",
            "docs/guides/a.md:10: names `Cordis`, retired: never built (#364): the DeepSeek harness is the built-in dsh adapter",
            "docs/guides/a.md:12: names `windows-`, retired: decision 0063: the hosts are Linux and macOS",
            "docs/guides/b.md: allows `winget`, which it no longer names",
        ]
    );
}

#[test]
fn a_quote_parts_a_retired_term_from_the_citation_beside_it() {
    let docs = [
        (
            "docs/guides/s.md",
            "> The hosts are Linux and macOS (decision 0063).\n>\n> winget install brokkr\n",
        ),
        (
            "docs/guides/i.md",
            "The hosts are Linux and macOS (decision 0063).\n> winget install brokkr\n",
        ),
        (
            "docs/guides/l.md",
            "> The hosts are Linux and macOS (decision 0063).\nwinget runs on lazily.\n\
             > > A deeper quote names Scoop.\n",
        ),
    ]
    .map(|(path, text)| doc(path, text).unwrap());
    let windows = |at: &str, term: &str| {
        format!("docs/guides/{at}: names `{term}`, retired: decision 0063: the hosts are Linux and macOS")
    };
    assert_eq!(
        retired_terms(&docs, &[]),
        [
            windows("s.md:3", "winget"),
            windows("i.md:2", "winget"),
            windows("l.md:3", "Scoop"),
        ]
    );
}
