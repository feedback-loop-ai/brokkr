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

/// Extract the first ` ```text ` fenced block and the first Markdown table from
/// one `## ` section body.
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

    let mut table = String::new();
    let mut in_table = false;
    for line in lines {
        if line.starts_with('|') {
            in_table = true;
            table.push_str(line);
            table.push('\n');
        } else if in_table {
            break;
        }
    }
    if table.is_empty() {
        return Err(HandsRefDrift::MissingTable {
            verb: verb.to_string(),
        });
    }
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

    let drifted_synopsis = committed.replace(
        "Usage: brokkr hands serve [OPTIONS] --workdir <WORKDIR> --artifacts <ARTIFACTS>",
        "Usage: brokkr hands serve [OPTIONS] --workdir <WORKDIR> --artifacts <ARTIFACTS> --reach <REACH>",
    );
    assert_ne!(drifted_synopsis, committed);
    assert_eq!(
        check_hands_reference(&cli, &drifted_synopsis),
        [HandsRefDrift::Synopsis {
            verb: "brokkr hands serve".to_string(),
            expected:
                "Usage: brokkr hands serve [OPTIONS] --workdir <WORKDIR> --artifacts <ARTIFACTS>"
                    .to_string(),
            actual: "Usage: brokkr hands serve [OPTIONS] --workdir <WORKDIR> --artifacts <ARTIFACTS> --reach <REACH>"
                .to_string(),
        }]
    );

    let drifted_table = committed.replace(
        "| `--spec <SPEC>` | `\"workspace\"` |",
        "| `--spec <SPEC>` |  |",
    );
    assert_ne!(drifted_table, committed);
    assert!(matches!(
        check_hands_reference(&cli, &drifted_table).as_slice(),
        [
            HandsRefDrift::OptionTable { verb: serve, .. },
            HandsRefDrift::OptionTable { verb: exec, .. },
        ] if serve == "brokkr hands serve" && exec == "brokkr hands exec"
    ));

    let missing_exec = committed
        .split_once("## `brokkr hands exec`")
        .expect("exec section")
        .0;
    assert_eq!(
        check_hands_reference(&cli, missing_exec),
        [HandsRefDrift::SectionHeadings {
            expected: vec![
                "brokkr hands serve".to_string(),
                "brokkr hands exec".to_string(),
            ],
            actual: vec!["brokkr hands serve".to_string()],
        }]
    );
}
