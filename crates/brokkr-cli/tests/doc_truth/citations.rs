//! The citation rule: every decision a doc, a source comment or a JSON
//! string calls proposed is proposed in `docs/decisions/README.md`.

use super::{blocks, is_living};
use crate::workspace_root::{read, workspace};
use crate::{decisions_index, rust_source, tracked_files};

/// A word without the punctuation around it, lowercased.
fn bare(word: &str) -> String {
    let word = word.trim_end_matches("'s").trim_end_matches("’s");
    word.trim_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase()
}

/// A word that is a four-digit decision number, its punctuation and the
/// target of a link it is the text of (`[0069](…)`) dropped.
fn number(word: Option<&&str>) -> Option<u16> {
    let word = word?;
    let word = bare(word.split_once(']').map_or(*word, |(text, _)| text));
    (word.len() == 4 && word.chars().all(|c| c.is_ascii_digit()))
        .then(|| word.parse().expect("four digits"))
}

/// The length of a Markdown link's `destination "title")`, the text after
/// its `](`, through the `)` that closes it: a destination of no whitespace
/// whose parentheses balance, and a title in `"`, `'` or `()` after it.
/// `None` when no link closes there, so the text is read as words.
fn destination(text: &str) -> Option<usize> {
    let mut depth = 0;
    let mut end = text.len();
    for (at, c) in text.char_indices() {
        match c {
            ')' if depth == 0 => return Some(at + 1),
            ')' => depth -= 1,
            '(' => depth += 1,
            c if c.is_whitespace() => {
                end = at;
                break;
            }
            _ => {}
        }
    }
    let titled = text[end..].trim_start();
    let close = match titled.chars().next()? {
        '"' => '"',
        '\'' => '\'',
        '(' => ')',
        _ => return None,
    };
    let after = titled[1..].split_once(close)?.1.trim_start();
    after.starts_with(')').then(|| text.len() - after.len() + 1)
}

/// A word's text after the Markdown link it holds, or the whole word.
fn after_link(word: &str) -> &str {
    word.find("](")
        .and_then(|at| Some(&word[at + 2 + destination(&word[at + 2..])?..]))
        .unwrap_or(word)
}

/// The words a citation is read in: parted at whitespace and after each
/// comma, so `0056,0069` lists two, but never inside a Markdown link, so
/// `[0069](a,b "Title")` is one word whatever its destination and title
/// hold.
fn words(text: &str) -> Vec<&str> {
    let (mut words, mut start, mut at) = (Vec::new(), None, 0);
    while let Some(c) = text[at..].chars().next() {
        if c.is_whitespace() {
            words.extend(start.take().map(|from| &text[from..at]));
            at += c.len_utf8();
            continue;
        }
        let from = *start.get_or_insert(at);
        let linked = text[at..]
            .strip_prefix("](")
            .and_then(destination)
            .map(|len| 2 + len);
        at += linked.unwrap_or(c.len_utf8());
        if linked.is_none() && c == ',' {
            words.push(&text[from..at]);
            start = None;
        }
    }
    words.extend(start.map(|from| &text[from..]));
    words
}

/// Whether `word` closes a citation: `)`, `.`, `;` or `:` after its
/// letters, or after the link it holds.
fn closing(word: &str) -> bool {
    let tail = after_link(word).rsplit(char::is_alphanumeric).next();
    tail.is_some_and(|tail| tail.contains([')', '.', ';', ':']))
}

/// The decision numbers a list starting at `words[from]` names (`0015`,
/// `0015 and 0069`, `0015, 0016, or 0069`, `0015,0069`), and the word that
/// ends it: none past a number word [`closing`] the citation.
fn listed(words: &[&str], from: usize) -> (Vec<u16>, Option<usize>) {
    let (mut numbers, mut last, mut at) = (Vec::new(), None, from);
    while let Some(cited) = number(words.get(at)) {
        numbers.push(cited);
        last = Some(at);
        let joined = words
            .get(at + 1)
            .is_some_and(|word| ["and", "or"].contains(&bare(word).as_str()));
        if closing(words[at]) || (!joined && !words[at].ends_with(',')) {
            break;
        }
        at += 1 + usize::from(joined);
    }
    (numbers, last)
}

/// Whether the citation ends at `words[at]`: the word is [`closing`], or
/// the next word opens another (`(`).
fn closes(words: &[&str], at: usize) -> bool {
    closing(words[at]) || words.get(at + 1).is_some_and(|next| next.starts_with('('))
}

/// Each decision `proposed` at `words[at]` names after it: `proposed
/// [decision|decisions] NNNN` or `proposed by|in|for [decision|decisions]
/// NNNN`, each number of a [`listed`] run, and none past a word that
/// [`closes`] the citation.
fn prefixed_proposed(words: &[&str], at: usize) -> Vec<u16> {
    let mut next = at;
    for joins in [&["by", "in", "for"][..], &["decision", "decisions"]] {
        let joined = words
            .get(next + 1)
            .is_some_and(|word| joins.contains(&bare(word).as_str()));
        if !closes(words, next) && joined {
            next += 1;
        }
    }
    match closes(words, next) {
        true => Vec::new(),
        false => listed(words, next + 1).0,
    }
}

/// Each decision a list after `decision` or `decisions` at `words[at]`
/// calls proposed by a trailing `, proposed` that proposes nothing else:
/// one [`prefixed_proposed`] reads no decision after.
fn trailed_proposed(words: &[&str], at: usize) -> Vec<u16> {
    let (cited, Some(last)) = listed(words, at + 1) else {
        return Vec::new();
    };
    let trailed = words[last].ends_with(',')
        && words
            .get(last + 1)
            .is_some_and(|word| bare(word) == "proposed")
        && prefixed_proposed(words, last + 1).is_empty();
    if trailed {
        cited
    } else {
        Vec::new()
    }
}

/// Each decision a text calls proposed: a [`prefixed_proposed`] citation
/// or a [`trailed_proposed`] one, in any case and across line breaks, in
/// its [`words`].
fn called_proposed(text: &str) -> Vec<u16> {
    let words = words(text);
    let mut found = Vec::new();
    for (at, word) in words.iter().enumerate() {
        let word = bare(word);
        if word == "proposed" {
            found.extend(prefixed_proposed(&words, at));
        } else if word == "decision" || word == "decisions" {
            found.extend(trailed_proposed(&words, at));
        }
    }
    found
}

/// The comment a shell line carries: the text after the first `#` that
/// opens a word, unless it is a shebang. Quotes are not read, so a `#`
/// opening a word inside them starts a comment too, which can only read
/// more.
fn shell_comment(line: &str) -> Option<&str> {
    let (at, _) = line.char_indices().find(|&(at, c)| {
        c == '#'
            && line[..at]
                .chars()
                .next_back()
                .is_none_or(char::is_whitespace)
    })?;
    let comment = &line[at + 1..];
    (!comment.starts_with('!')).then_some(comment)
}

/// Each string a JSON text holds, its keys among them, decoded, with the
/// line it opens on; a text that is not JSON is refused by its parser.
fn json_strings(text: &str) -> Result<Vec<(usize, String)>, serde_json::Error> {
    serde_json::from_str::<serde::de::IgnoredAny>(text)?;
    let (mut found, mut line, mut open, mut escaped) = (Vec::new(), 1, None, false);
    for (at, c) in text.char_indices() {
        match (open, c) {
            (None, '"') => open = Some(at),
            (None, '\n') => line += 1,
            (None, _) => {}
            (Some(_), _) if escaped => escaped = false,
            (Some(_), '\\') => escaped = true,
            (Some(start), '"') => {
                found.push((line, serde_json::from_str(&text[start..=at])?));
                open = None;
            }
            (Some(_), _) => {}
        }
    }
    Ok(found)
}

/// The prose of a tracked file these rules hold: a doc's blocks, each
/// string of a JSON text alone, a Rust source's comments, a script's
/// [`shell_comment`]s, each a run of adjacent lines with the line it
/// starts on.
fn prose(path: &str, text: &str) -> Result<Vec<(usize, String)>, String> {
    let lines: Vec<(usize, String)> = match path.rsplit_once('.').map(|(_, ext)| ext) {
        Some("md") => {
            return Ok(blocks(text)?
                .into_iter()
                .map(|block| (block.first, block.text))
                .collect());
        }
        Some("json") => return json_strings(text).map_err(|why| why.to_string()),
        Some("rs") => rust_source::comments(text)?
            .into_iter()
            .map(|(at, comment)| {
                let lines: Vec<&str> = comment
                    .lines()
                    .map(|line| {
                        line.trim()
                            .trim_end_matches("*/")
                            .trim_start_matches(['/', '*', '!'])
                    })
                    .collect();
                (at, lines.join("\n"))
            })
            .collect(),
        _ => text
            .lines()
            .zip(1..)
            .filter_map(|(line, at)| Some((at, shell_comment(line)?.to_string())))
            .collect(),
    };
    let mut runs: Vec<(usize, String, usize)> = Vec::new();
    for (at, comment) in lines {
        let end = at + comment.matches('\n').count();
        match runs.last_mut() {
            Some((_, run, last)) if *last + 1 == at => {
                run.push('\n');
                run.push_str(&comment);
                *last = end;
            }
            _ => runs.push((at, comment, end)),
        }
    }
    Ok(runs.into_iter().map(|(at, run, _)| (at, run)).collect())
}

/// Each decision `texts` call proposed that the index does not hold as
/// proposed, and each text that cannot be read.
fn misnamed_proposals(
    texts: &[(String, String)],
    statuses: &std::collections::BTreeMap<u16, String>,
) -> Vec<String> {
    let mut found = Vec::new();
    for (path, text) in texts {
        let runs = match prose(path, text) {
            Ok(runs) => runs,
            Err(why) => {
                found.push(format!("{path}: cannot be read: {why}"));
                continue;
            }
        };
        for (line, run) in runs {
            for cited in called_proposed(&run) {
                match statuses.get(&cited).map(String::as_str) {
                    Some("proposed") => {}
                    Some(status) => found.push(format!(
                        "{path}:{line}: calls decision {cited:04} proposed; the index rules it {status}"
                    )),
                    None => found.push(format!(
                        "{path}:{line}: calls decision {cited:04} proposed; the index holds no such decision"
                    )),
                }
            }
        }
    }
    found
}

#[test]
fn every_decision_called_proposed_is_proposed() {
    let root = workspace();
    let texts: Vec<(String, String)> =
        tracked_files::tracked(&root, &["*.md", "*.rs", "*.sh", "*.json"])
            .into_iter()
            .filter(|path| is_living(path))
            .map(|path| {
                let text = read(&path);
                (path, text)
            })
            .collect();
    for known in [
        "README.md",
        "adapters/codex.json",
        "crates/brokkr-protocol/src/adapters.rs",
        "scripts/coverage-exact.sh",
    ] {
        assert!(
            texts.iter().any(|(path, _)| path == known),
            "{known} is not read"
        );
    }
    assert_eq!(
        misnamed_proposals(&texts, &decisions_index::cited_statuses()),
        Vec::<String>::new(),
        "a living doc, a source comment or a JSON string calls a decision proposed that the index does not"
    );
}

/// The index the planted citations are held to: 0069 accepted, 0056 and
/// 0075 proposed.
fn planted_statuses() -> std::collections::BTreeMap<u16, String> {
    [(56, "proposed"), (69, "accepted"), (75, "proposed")]
        .into_iter()
        .map(|(number, status)| (number, status.to_string()))
        .collect()
}

#[test]
fn a_decision_called_proposed_is_held_to_the_index() {
    let statuses = planted_statuses();
    let planted = [
        (
            "crates/a/src/lib.rs",
            "/// The stamp (proposed\n/// decision 0056 ruling 1).\nfn a() {}\n\
             // proposed decision 0069 ruling 2\nconst S: &str = \"proposed decision 0069\";\n\
             /* Proposed 0071's tail */\n",
        ),
        (
            "docs/a.md",
            "Costs are proposed in decision 0069, which only the operator\n\
             rules (decision 0075, proposed). A proposed decision is a draft;\n\
             (decision 0069, proposed 0056 ruling 4) (decision 0069, proposed decision 0075)\n\
             the proposed 2026-09-28 addendum and #403 are no number.\n\
             \n\
             ```\n\
             (decision 0069, proposed)\n\
             ```\n\
             \n\
             proposed decision [0069](0069-x.md); (decision [0069](x), proposed).\n\
             \n\
             proposed decisions 0056 and 0069; (decisions 0075, 0056, and 0069, proposed).\n\
             \n\
             (decision 0069, proposed decisions 0056 and 0075) (decision 0069, proposed by\n\
             decision 0056) (decision 0069, proposed in 0075) (decision 0069, proposed by us).\n\
             \n\
             (decision 0069, proposed). Decision 0075 describes the stamp.\n\
             \n\
             (decision 0069, proposed) (decision 0075).\n\
             \n\
             Decision 0069, proposed (decision 0075 describes the stamp).\n\
             \n\
             > A proposed decision\n\
             > 0069; (decision 0069,\n\
             > proposed); proposed decisions 0056 and\n\
             > 0069.\n\
             \n\
             proposed decisions 0056,0069; (decisions 0075,0069, proposed).\n",
        ),
        (
            "scripts/a.sh",
            "#!/bin/bash\n# Proposed by decision 0069.\necho proposed decision 0069\n\
             true # proposed decision 0069\necho \"#a\" proposed decision 0069 b#c\n",
        ),
        (
            "crates/a/src/open.rs",
            "fn f() { let s = \"never closed; }\n",
        ),
        (
            "adapters/a.json",
            "{\"notes\": [\"motivate it (proposed\",\n\
             \"decision 0069)\", \"a \\\"proposed decision 0069\\\" quote\"],\n\
             \"proposed decision 0056\": \"a\\\\\", \"b\": \"(decision 0069, proposed)\"}\n",
        ),
        ("adapters/open.json", "{\"a\": \"proposed decision 0069\""),
    ];
    let texts: Vec<(String, String)> = planted
        .iter()
        .map(|(path, text)| (path.to_string(), text.to_string()))
        .collect();
    assert_eq!(
        misnamed_proposals(&texts, &statuses),
        [
            "crates/a/src/lib.rs:4: calls decision 0069 proposed; the index rules it accepted",
            "crates/a/src/lib.rs:6: calls decision 0071 proposed; the index holds no such decision",
            "docs/a.md:1: calls decision 0069 proposed; the index rules it accepted",
            "docs/a.md:7: calls decision 0069 proposed; the index rules it accepted",
            "docs/a.md:10: calls decision 0069 proposed; the index rules it accepted",
            "docs/a.md:10: calls decision 0069 proposed; the index rules it accepted",
            "docs/a.md:12: calls decision 0069 proposed; the index rules it accepted",
            "docs/a.md:12: calls decision 0069 proposed; the index rules it accepted",
            "docs/a.md:14: calls decision 0069 proposed; the index rules it accepted",
            "docs/a.md:17: calls decision 0069 proposed; the index rules it accepted",
            "docs/a.md:19: calls decision 0069 proposed; the index rules it accepted",
            "docs/a.md:21: calls decision 0069 proposed; the index rules it accepted",
            "docs/a.md:23: calls decision 0069 proposed; the index rules it accepted",
            "docs/a.md:23: calls decision 0069 proposed; the index rules it accepted",
            "docs/a.md:23: calls decision 0069 proposed; the index rules it accepted",
            "docs/a.md:28: calls decision 0069 proposed; the index rules it accepted",
            "docs/a.md:28: calls decision 0069 proposed; the index rules it accepted",
            "scripts/a.sh:2: calls decision 0069 proposed; the index rules it accepted",
            "scripts/a.sh:4: calls decision 0069 proposed; the index rules it accepted",
            "crates/a/src/open.rs: cannot be read: an unterminated string",
            "adapters/a.json:2: calls decision 0069 proposed; the index rules it accepted",
            "adapters/a.json:3: calls decision 0069 proposed; the index rules it accepted",
            "adapters/open.json: cannot be read: EOF while parsing an object at line 1 column 30",
        ]
    );
}

#[test]
fn a_proposed_citation_ends_where_it_closes_and_reads_a_link_whole() {
    let planted = [(
        "docs/c.md".to_string(),
        "The draft (proposed decision 0056), and 0069 is accepted.\n\
         A proposed decision. 0069 is accepted; proposed by. 0069 too; proposed by. Decision 0069.\n\
         \n\
         proposed decisions 0056 and 0069; proposed decisions [0056](a), [0069](b).\n\
         \n\
         (decision [0069](https://example.invalid/a,b), proposed).\n\
         (decision [0069](0069-x.md \"Title\"), proposed).\n\
         proposed decision [0069](x 'a (b) c'); proposed decision [0069](a(b)c).\n"
            .to_string(),
    )];
    let accepted = |line: usize| {
        format!("docs/c.md:{line}: calls decision 0069 proposed; the index rules it accepted")
    };
    assert_eq!(
        misnamed_proposals(&planted, &planted_statuses()),
        [4, 4, 6, 6, 6, 6].map(accepted)
    );
    assert_eq!(
        words("a,b [0069](x,y \"T, u\"), c [d](e"),
        ["a,", "b", "[0069](x,y \"T, u\"),", "c", "[d](e"]
    );
}
