//! `docs/reference/hands.md`, held to `Cli::command()` (#1277, decision 0071
//! ruling 5).
//!
//! `brokkr hands` is hidden from `docs/reference/cli.md`, so its synopses and
//! option tables live in `docs/reference/hands.md`. This module derives the
//! expected `Usage:` synopsis and option table for every subcommand of
//! `brokkr hands` directly from `Cli::command()` and fails whenever the
//! committed page drifts.

use std::fmt;

use clap::{Arg, Command};

use super::{built, default_values, description, listed, spelled, undescribed_in, workspace};

/// Where the hands reference page lives, from the workspace root.
const HANDS_REFERENCE: &str = "docs/reference/hands.md";

/// A parsed `## ` section from `docs/reference/hands.md`: its verb heading,
/// its `text` usage block, and its option table lines.
#[derive(Debug, PartialEq, Eq)]
struct ParsedSection {
    verb: String,
    usage: String,
    table: String,
}

/// How `docs/reference/hands.md` drifted from `Cli::command()`.
#[derive(Debug, PartialEq, Eq)]
enum HandsRefDrift {
    UnclosedUsage {
        verb: String,
    },
    MissingUsage {
        verb: String,
    },
    MissingTable {
        verb: String,
    },
    SectionHeadings {
        expected: Vec<String>,
        actual: Vec<String>,
    },
    Synopsis {
        verb: String,
        expected: String,
        actual: String,
    },
    OptionTable {
        verb: String,
        expected: String,
        actual: String,
    },
}

impl fmt::Display for HandsRefDrift {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnclosedUsage { verb } => write!(f, "{verb}: unclosed ```text usage block"),
            Self::MissingUsage { verb } => write!(f, "{verb}: missing ```text usage block"),
            Self::MissingTable { verb } => write!(f, "{verb}: missing option table"),
            Self::SectionHeadings { expected, actual } => {
                write!(
                    f,
                    "section headings differ: expected {expected:?}, found {actual:?}"
                )
            }
            Self::Synopsis {
                verb,
                expected,
                actual,
            } => write!(
                f,
                "{verb}: usage synopsis drifted:\n  expected: {expected}\n  actual:   {actual}"
            ),
            Self::OptionTable {
                verb,
                expected,
                actual,
            } => write!(
                f,
                "{verb}: option table drifted:\nexpected:\n{expected}actual:\n{actual}"
            ),
        }
    }
}

/// Every non-help subcommand under `brokkr hands`, in clap definition order.
fn hands_verbs(cli: &Command) -> Vec<&Command> {
    cli.find_subcommand("hands")
        .expect("brokkr hands subcommand")
        .get_subcommands()
        .filter(|verb| verb.get_name() != "help")
        .collect()
}

/// One argument's row in `docs/reference/hands.md`'s option table.
fn hands_argument_row(arg: &Arg) -> String {
    format!(
        "| `{}` | {} | {} |\n",
        spelled(arg),
        default_values(arg),
        description(arg),
    )
}

/// The expected synopsis and option table for one `brokkr hands` subcommand,
/// derived from the built clap `Command`.
fn expected_section(verb: &Command) -> ParsedSection {
    let path = verb.get_bin_name().expect("a built command names its path");
    let usage = verb.clone().render_usage().to_string();
    let mut table = String::from("| Argument | Default | Description |\n| --- | --- | --- |\n");
    for arg in listed(verb) {
        table.push_str(&hands_argument_row(arg));
    }
    ParsedSection {
        verb: path.to_string(),
        usage: usage.trim_end().to_string(),
        table,
    }
}

/// Strip up to three leading ASCII spaces, as CommonMark block openers allow.
fn strip_up_to_three_spaces(line: &str) -> &str {
    let spaces = line.bytes().take(3).take_while(|&b| b == b' ').count();
    &line[spaces..]
}

/// Remove a single trailing unescaped `|` from `s`, if present.
fn strip_unescaped_trailing_pipe(s: &str) -> Option<&str> {
    let rest = s.strip_suffix('|')?;
    let backslashes = rest.bytes().rev().take_while(|&b| b == b'\\').count();
    (backslashes % 2 == 0).then_some(rest)
}

/// Split one GFM table row into cells on unescaped `|` separators, ignoring an
/// optional single leading or trailing pipe.
fn split_gfm_row(line: &str) -> Vec<&str> {
    let mut trimmed = line.trim_matches([' ', '\t']);
    if let Some(rest) = trimmed.strip_prefix('|') {
        trimmed = rest;
    }
    if let Some(rest) = strip_unescaped_trailing_pipe(trimmed) {
        trimmed = rest;
    }
    let bytes = trimmed.as_bytes();
    let mut cells = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            i += 2;
            continue;
        }
        if bytes[i] == b'|' {
            cells.push(&trimmed[start..i]);
            start = i + 1;
        }
        i += 1;
    }
    cells.push(&trimmed[start..]);
    cells
}

/// Whether `cell` is a valid GFM table delimiter cell (`:?-+:?`).
fn is_delimiter_cell(cell: &str) -> bool {
    let trimmed = cell.trim_matches([' ', '\t']);
    let inner = trimmed.strip_prefix(':').unwrap_or(trimmed);
    let inner = inner.strip_suffix(':').unwrap_or(inner);
    !inner.is_empty() && inner.bytes().all(|b| b == b'-')
}

/// Column count when `line` is a GFM table delimiter row (requiring at least
/// one pipe so a setext heading underline `---` is not misread as a table).
fn gfm_delimiter_columns(line: &str) -> Option<usize> {
    if !line.contains('|') {
        return None;
    }
    let cells = split_gfm_row(line);
    (!cells.is_empty() && cells.iter().all(|cell| is_delimiter_cell(cell))).then_some(cells.len())
}

/// Opening marker `(char, length)` if `line` opens a CommonMark fenced code block.
fn fence_marker(line: &str) -> Option<(u8, usize)> {
    let rest = strip_up_to_three_spaces(line);
    let first = *rest.as_bytes().first()?;
    if first != b'`' && first != b'~' {
        return None;
    }
    let count = rest.bytes().take_while(|&b| b == first).count();
    if count < 3 || (first == b'`' && rest[count..].contains('`')) {
        return None;
    }
    Some((first, count))
}

/// Whether `line` closes a fenced code block opened with `marker`.
fn closes_fence(line: &str, marker: (u8, usize)) -> bool {
    let rest = strip_up_to_three_spaces(line);
    let count = rest.bytes().take_while(|&b| b == marker.0).count();
    count >= marker.1 && rest[count..].trim_matches([' ', '\t']).is_empty()
}

/// Whether `rest` (after stripping up to 3 spaces) is a CommonMark thematic break.
fn is_thematic_break(rest: &str) -> bool {
    let Some(&marker) = rest.as_bytes().first() else {
        return false;
    };
    if !matches!(marker, b'-' | b'*' | b'_') {
        return false;
    }
    let mut count = 0;
    for byte in rest.bytes() {
        if byte == marker {
            count += 1;
        } else if !matches!(byte, b' ' | b'\t') {
            return false;
        }
    }
    count >= 3
}

/// Whether `rest` (after stripping up to 3 spaces) opens an ATX heading.
fn is_atx_heading(rest: &str) -> bool {
    let hashes = rest.bytes().take_while(|&b| b == b'#').count();
    (1..=6).contains(&hashes) && matches!(rest.as_bytes().get(hashes), None | Some(b' ' | b'\t'))
}

/// Whether `rest` (after stripping up to 3 spaces) opens a list item.
fn is_list_item(rest: &str) -> bool {
    let bytes = rest.as_bytes();
    if matches!(bytes.first(), Some(b'-' | b'*' | b'+'))
        && matches!(bytes.get(1), None | Some(b' ' | b'\t'))
    {
        return true;
    }
    let digits = bytes.iter().take_while(|b| b.is_ascii_digit()).count();
    (1..=9).contains(&digits)
        && matches!(bytes.get(digits), Some(b'.' | b')'))
        && matches!(bytes.get(digits + 1), None | Some(b' ' | b'\t'))
}

/// Whether `line` ends a GFM table (blank line or interrupting block construct).
fn interrupts_gfm_table(line: &str) -> bool {
    if line.trim().is_empty() {
        return true;
    }
    let rest = strip_up_to_three_spaces(line);
    rest.starts_with('>')
        || is_atx_heading(rest)
        || fence_marker(line).is_some()
        || is_thematic_break(rest)
        || is_list_item(rest)
}

/// Extract the first GFM table from `lines`, reading every row GFM renders as
/// part of that table whether or not it carries a leading pipe.
fn extract_first_table(lines: &[&str]) -> Option<String> {
    let mut in_fence: Option<(u8, usize)> = None;
    for i in 0..lines.len() {
        if let Some(marker) = in_fence {
            if closes_fence(lines[i], marker) {
                in_fence = None;
            }
            continue;
        }
        if let Some(marker) = fence_marker(lines[i]) {
            in_fence = Some(marker);
            continue;
        }
        let Some(next) = lines.get(i + 1) else {
            break;
        };
        let Some(columns) = gfm_delimiter_columns(next) else {
            continue;
        };
        if interrupts_gfm_table(lines[i]) || split_gfm_row(lines[i]).len() != columns {
            continue;
        }
        let mut table = String::new();
        table.push_str(lines[i]);
        table.push('\n');
        table.push_str(next);
        table.push('\n');
        for row in &lines[i + 2..] {
            if interrupts_gfm_table(row) {
                break;
            }
            table.push_str(row);
            table.push('\n');
        }
        return Some(table);
    }
    None
}

/// Extract the first ` ```text ` fenced block and the first GFM table from one
/// `## ` section body.
fn parse_section_body(verb: &str, body: &str) -> Result<ParsedSection, HandsRefDrift> {
    let mut lines = body.lines();
    let mut usage_lines = Vec::new();
    let mut found_usage = false;
    while let Some(line) = lines.next() {
        if line.trim() == "```text" {
            let mut closed = false;
            for inner in lines.by_ref() {
                if inner.trim() == "```" {
                    closed = true;
                    break;
                }
                usage_lines.push(inner);
            }
            if !closed {
                return Err(HandsRefDrift::UnclosedUsage {
                    verb: verb.to_string(),
                });
            }
            found_usage = true;
            break;
        }
    }
    if !found_usage {
        return Err(HandsRefDrift::MissingUsage {
            verb: verb.to_string(),
        });
    }

    let remaining: Vec<&str> = lines.collect();
    let Some(table) = extract_first_table(&remaining) else {
        return Err(HandsRefDrift::MissingTable {
            verb: verb.to_string(),
        });
    };
    Ok(ParsedSection {
        verb: verb.to_string(),
        usage: usage_lines.join("\n"),
        table,
    })
}

/// Parse every `## ` section in `page` into its verb path, usage synopsis, and
/// option table.
fn parse_sections(page: &str) -> Result<Vec<ParsedSection>, HandsRefDrift> {
    let mut raw: Vec<(String, String)> = Vec::new();
    for line in page.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            let verb = heading.trim().trim_matches('`').to_string();
            raw.push((verb, String::new()));
        } else if let Some((_, body)) = raw.last_mut() {
            body.push_str(line);
            body.push('\n');
        }
    }
    raw.into_iter()
        .map(|(verb, body)| parse_section_body(&verb, &body))
        .collect()
}

/// Check `page` against the built `brokkr hands` subcommands, returning every
/// drift found.
fn check_hands_reference(cli: &Command, page: &str) -> Vec<HandsRefDrift> {
    let expected: Vec<ParsedSection> = hands_verbs(cli).into_iter().map(expected_section).collect();
    let actual = match parse_sections(page) {
        Ok(sections) => sections,
        Err(err) => return vec![err],
    };

    let expected_verbs: Vec<String> = expected.iter().map(|s| s.verb.clone()).collect();
    let actual_verbs: Vec<String> = actual.iter().map(|s| s.verb.clone()).collect();
    if actual_verbs != expected_verbs {
        return vec![HandsRefDrift::SectionHeadings {
            expected: expected_verbs,
            actual: actual_verbs,
        }];
    }

    let mut drifts = Vec::new();
    for (exp, act) in expected.iter().zip(actual.iter()) {
        if act.usage != exp.usage {
            drifts.push(HandsRefDrift::Synopsis {
                verb: exp.verb.clone(),
                expected: exp.usage.clone(),
                actual: act.usage.clone(),
            });
        }
        if act.table != exp.table {
            drifts.push(HandsRefDrift::OptionTable {
                verb: exp.verb.clone(),
                expected: exp.table.clone(),
                actual: act.table.clone(),
            });
        }
    }
    drifts
}

#[test]
fn the_hands_reference_matches_clap() {
    let cli = built();
    let committed = std::fs::read_to_string(workspace().join(HANDS_REFERENCE))
        .expect("docs/reference/hands.md reads");
    let drifts = check_hands_reference(&cli, &committed);
    assert!(
        drifts.is_empty(),
        "{HANDS_REFERENCE} differs from Cli::command():\n{}",
        drifts
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    );
    let undescribed = undescribed_in(hands_verbs(&cli));
    assert!(
        undescribed.is_empty(),
        "arguments {HANDS_REFERENCE} would list with no description:\n{}",
        undescribed.join("\n")
    );
}

#[test]
fn the_hands_reference_check_catches_synopsis_table_and_section_drift() {
    let cli = built();
    let committed = std::fs::read_to_string(workspace().join(HANDS_REFERENCE))
        .expect("docs/reference/hands.md reads");
    let verbs = hands_verbs(&cli);
    let expected: Vec<ParsedSection> = verbs.iter().map(|verb| expected_section(verb)).collect();
    let serve = verbs
        .iter()
        .copied()
        .find(|verb| verb.get_name() == "serve")
        .expect("hands serve subcommand");
    let serve_expected = expected_section(serve);

    // Derive the mutated synopsis from Cli::command() rather than repeating it.
    let mutated_synopsis = format!("{} --drifted <DRIFTED>", serve_expected.usage);
    let drifted_synopsis = committed.replacen(&serve_expected.usage, &mutated_synopsis, 1);
    assert_ne!(drifted_synopsis, committed);
    assert_eq!(
        check_hands_reference(&cli, &drifted_synopsis),
        [HandsRefDrift::Synopsis {
            verb: serve_expected.verb.clone(),
            expected: serve_expected.usage.clone(),
            actual: mutated_synopsis,
        }]
    );

    // Derive the `--spec` default cell from Cli::command() rather than a literal.
    let spec_arg = listed(serve)
        .into_iter()
        .find(|arg| arg.get_id() == "spec")
        .expect("hands serve --spec");
    let spec_cell = format!("| `{}` | {} |", spelled(spec_arg), default_values(spec_arg));
    let empty_default_cell = format!("| `{}` |  |", spelled(spec_arg));
    assert_ne!(spec_cell, empty_default_cell, "serve --spec has a default");
    let drifted_table = committed.replace(&spec_cell, &empty_default_cell);
    assert_ne!(drifted_table, committed);
    let expected_drifted_verbs: Vec<String> = expected
        .iter()
        .filter(|section| section.table.contains(&spec_cell))
        .map(|section| section.verb.clone())
        .collect();
    assert!(!expected_drifted_verbs.is_empty());
    let actual_drifted_verbs: Vec<String> = check_hands_reference(&cli, &drifted_table)
        .into_iter()
        .map(|drift| match drift {
            HandsRefDrift::OptionTable { verb, .. } => verb,
            other => panic!("expected OptionTable drift, got {other:?}"),
        })
        .collect();
    assert_eq!(actual_drifted_verbs, expected_drifted_verbs);

    let expected_verbs: Vec<String> = expected.iter().map(|s| s.verb.clone()).collect();
    let last_verb = expected_verbs.last().expect("at least two hands verbs");
    let missing_last = committed
        .split_once(&format!("## `{last_verb}`"))
        .expect("last verb section")
        .0;
    assert_eq!(
        check_hands_reference(&cli, missing_last),
        [HandsRefDrift::SectionHeadings {
            expected: expected_verbs.clone(),
            actual: expected_verbs[..expected_verbs.len() - 1].to_vec(),
        }]
    );
}

#[test]
fn the_hands_reference_check_reads_gfm_table_rows_without_leading_pipe() {
    let cli = built();
    let committed = std::fs::read_to_string(workspace().join(HANDS_REFERENCE))
        .expect("docs/reference/hands.md reads");
    let serve = hands_verbs(&cli)
        .into_iter()
        .find(|verb| verb.get_name() == "serve")
        .expect("hands serve subcommand");
    let serve_expected = expected_section(serve);

    // 1. An extra option row without a leading pipe appended directly to the
    //    table (with a wrong default) is read as part of the table and fails.
    let extra_row = "`--extra <EXTRA>` | `\"wrong\"` | Extra option\n";
    let appended_table = format!("{}{extra_row}", serve_expected.table);
    let with_appended = committed.replacen(&serve_expected.table, &appended_table, 1);
    assert_ne!(with_appended, committed);
    assert_eq!(
        check_hands_reference(&cli, &with_appended),
        [HandsRefDrift::OptionTable {
            verb: serve_expected.verb.clone(),
            expected: serve_expected.table.clone(),
            actual: appended_table,
        }]
    );

    // 2. A mid-table option row rewritten without a leading pipe (and a wrong
    //    default) does not truncate the table before following rows.
    let spec_arg = listed(serve)
        .into_iter()
        .find(|arg| arg.get_id() == "spec")
        .expect("hands serve --spec");
    let spec_row = hands_argument_row(spec_arg);
    let pipeless_spec_row = format!(
        "`{}` | `\"wrong\"` | {}\n",
        spelled(spec_arg),
        description(spec_arg),
    );
    let mid_table = serve_expected
        .table
        .replacen(&spec_row, &pipeless_spec_row, 1);
    let with_mid_pipeless = committed.replacen(&serve_expected.table, &mid_table, 1);
    assert_ne!(with_mid_pipeless, committed);
    assert_eq!(
        check_hands_reference(&cli, &with_mid_pipeless),
        [HandsRefDrift::OptionTable {
            verb: serve_expected.verb.clone(),
            expected: serve_expected.table.clone(),
            actual: mid_table,
        }]
    );

    // 3. A whole GFM table with no leading or trailing pipes on header,
    //    delimiter, or rows is still recognized as the section's option table
    //    (reporting OptionTable drift rather than MissingTable).
    let pipeless_table: String = serve_expected
        .table
        .lines()
        .map(|line| {
            let stripped = line.trim_start_matches('|').trim_end_matches('|').trim();
            format!("{stripped}\n")
        })
        .collect();
    let with_pipeless_table = committed.replacen(&serve_expected.table, &pipeless_table, 1);
    assert_ne!(with_pipeless_table, committed);
    assert_eq!(
        check_hands_reference(&cli, &with_pipeless_table),
        [HandsRefDrift::OptionTable {
            verb: serve_expected.verb,
            expected: serve_expected.table,
            actual: pipeless_table,
        }]
    );
}
