//! Decision 0064 ruling 6's listing of one hearth: its heading counts the
//! two wants, its closing line tallies the endings, and `--ending`,
//! `--needs-operator` and `--hide-settled` narrow it, each read by the
//! verb from the journal as `brokkr runs` reads it.

use std::path::{Path, PathBuf};

use brokkr_core::ending::Ending;
use clap::Parser;
use serde_json::Value;

use super::Filter;
use crate::fleet::page::tests::{folded, now, plant, stamp, Shape, HOUR, MINUTE, STYLE};
use crate::fleet::Called;

/// A hearth holding a running run, a parked one, a shipped one and one
/// whose journal does not fold, each last active an hour ago.
fn hearth() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let at = stamp(now() - HOUR);
    folded(&db, |conn| {
        plant(conn, "live", Shape::Running, &at);
        plant(conn, "parked", Shape::Parked, &at);
        plant(conn, "done", Shape::Finished, &at);
        plant(conn, "poisoned", Shape::Refused, &at);
    });
    (dir, db)
}

/// What `brokkr runs` prints of the hearth at `db` that `filter` keeps,
/// as JSON when `json` asks.
fn printed(db: &Path, filter: Filter, json: bool) -> String {
    paged(db, filter, (json, None, None))
}

/// What `brokkr runs` prints of the hearth at `db` that `filter` keeps,
/// `asked` as JSON or not and of a page or every run.
fn paged(
    db: &Path,
    filter: Filter,
    asked: (bool, Option<u32>, Option<&brokkr_view::Cursor>),
) -> String {
    let mut out = Vec::new();
    let flags = (filter, Called::default());
    crate::verbs::readouts::hearth_runs((db, "brokkr"), asked, flags, &STYLE, &mut out).unwrap();
    String::from_utf8(out).unwrap()
}

/// The table's heading, closing and next-page lines whole, its journal
/// written `<db>`, and each row as its gutter and its id: the clock-free
/// lines.
fn lines_of(db: &Path, text: &str) -> Vec<String> {
    let text = text.replace(&db.display().to_string(), "<db>");
    let whole = ["brokkr · ", "endings", "next page"];
    text.lines()
        .filter(|line| !line.starts_with("    "))
        .map(
            |line| match whole.iter().any(|head| line.starts_with(head)) {
                true => line.to_string(),
                false => {
                    let (gutter, row) = line.split_at(2);
                    format!("{gutter}{}", row.split(' ').next().unwrap())
                }
            },
        )
        .collect()
}

/// [`lines_of`] the listing of every run `filter` keeps.
fn shape(db: &Path, filter: Filter) -> Vec<String> {
    lines_of(db, &printed(db, filter, false))
}

/// Every run, under the heading that counts what wants the operator and
/// over the tally of how they ended: no listing hides a settled run.
#[test]
fn the_listing_is_headed_by_its_counts_and_closed_by_its_tally() {
    let (_dir, db) = hearth();
    assert_eq!(
        shape(&db, Filter::default()),
        [
            "brokkr · 4 runs · 1 wants a ruling · 1 wants an action · <db>",
            "> poisoned",
            "! parked",
            "  live",
            "  done",
            "endings  1 shipped",
        ]
    );
}

/// Each filter narrows the listing and its counts alike: `--ending` to
/// the runs that ended so, `--needs-operator` to the `!` and `>` rows,
/// `--hide-settled` past every settled row; in the table and the JSON.
#[test]
fn each_filter_narrows_the_listing_and_its_counts() {
    let (_dir, db) = hearth();
    let shipped = Filter {
        ending: Some(Ending::Shipped),
        ..Filter::default()
    };
    assert_eq!(
        shape(&db, shipped),
        [
            "brokkr · 1 run · 0 want a ruling · 0 want an action · <db>",
            "  done",
            "endings  1 shipped",
        ]
    );
    let json: Value = serde_json::from_str(&printed(&db, shipped, true)).unwrap();
    assert_eq!(
        (&json["count"], &json["runs"][0]["run_id"]),
        (&1.into(), &"done".into())
    );
    let wanting = Filter {
        needs_operator: true,
        ..Filter::default()
    };
    assert_eq!(
        shape(&db, wanting),
        [
            "brokkr · 2 runs · 1 wants a ruling · 1 wants an action · <db>",
            "> poisoned",
            "! parked",
        ]
    );
    let unsettled = Filter {
        hide_settled: true,
        ..Filter::default()
    };
    assert_eq!(
        shape(&db, unsettled),
        [
            "brokkr · 3 runs · 1 wants a ruling · 1 wants an action · <db>",
            "> poisoned",
            "! parked",
            "  live",
        ]
    );
}

/// `brokkr runs` as parsed from `argv`, or clap's refusal.
fn parsed(argv: &[&str]) -> Result<crate::cli_args::RunsArgs, clap::Error> {
    crate::Cli::try_parse_from(argv).map(|cli| match cli.command {
        crate::Cmd::Runs(args) => args,
        _ => panic!("`brokkr runs` parses as itself"),
    })
}

/// The three flags parse to the filter they name, and a word outside the
/// twelve is a usage error that names every one of them.
#[test]
fn the_filters_parse_and_an_unknown_ending_is_refused_naming_the_twelve() {
    let args = parsed(&[
        "brokkr",
        "runs",
        "--ending",
        "delivered-elsewhere",
        "--needs-operator",
        "--hide-settled",
    ])
    .unwrap();
    assert_eq!(
        (args.ending, args.needs_operator, args.hide_settled),
        (Some(Ending::DeliveredElsewhere), true, true)
    );
    let unflagged = parsed(&["brokkr", "runs"]).unwrap();
    assert_eq!(
        (
            unflagged.ending,
            unflagged.needs_operator,
            unflagged.hide_settled
        ),
        (None, false, false)
    );
    let refused = parsed(&["brokkr", "runs", "--ending", "stopped"])
        .err()
        .unwrap();
    assert_eq!(refused.kind(), clap::error::ErrorKind::ValueValidation);
    assert!(
        refused.to_string().contains(
            "invalid value 'stopped' for '--ending <ENDING>': 'stopped' is not an ending; an \
             ending is one of shipped, shipped-flagged, refused, failed, blocked, exhausted, \
             superseded, infrastructure, delivered-elsewhere, withdrawn, operator-stopped, \
             unclassified"
        ),
        "{refused}"
    );
    let one = parsed(&["brokkr", "runs", "--run", "r", "--hide-settled"])
        .err()
        .unwrap();
    assert_eq!(one.kind(), clap::error::ErrorKind::ArgumentConflict);
}

/// A page of the listing and `--run`'s one row are framed as the whole
/// listing is: the heading counts the rows it shows, and the closing line
/// tallies how they ended, before the next page's command.
#[test]
fn a_page_and_one_run_are_headed_and_tallied_as_the_listing_is() {
    let (_dir, db) = hearth();
    let first = paged(&db, Filter::default(), (false, Some(2), None));
    let next = format!(
        "next page  brokkr runs --limit 2 --after needs-you:{}:parked",
        now() - HOUR
    );
    assert_eq!(
        lines_of(&db, &first),
        [
            "brokkr · 2 runs · 1 wants a ruling · 1 wants an action · <db>",
            "> poisoned",
            "! parked",
            next.as_str(),
        ]
    );
    let after: brokkr_view::Cursor = next.rsplit(' ').next().unwrap().parse().unwrap();
    let second = paged(&db, Filter::default(), (false, Some(2), Some(&after)));
    assert_eq!(
        lines_of(&db, &second),
        [
            "brokkr · 2 runs · 0 want a ruling · 0 want an action · <db>",
            "  live",
            "  done",
            "endings  1 shipped",
        ]
    );
    let mut out = Vec::new();
    let one = (db.as_path(), "brokkr");
    crate::verbs::readouts::hearth_run(one, ("done", false), &STYLE, &mut out).unwrap();
    assert_eq!(
        lines_of(&db, &String::from_utf8(out).unwrap()),
        [
            "brokkr · 1 run · 0 want a ruling · 0 want an action · <db>",
            "  done",
            "endings  1 shipped",
        ]
    );
}

/// A hearth of two runs of each group, interleaved by when each was last
/// active: shipped, parked and running, and one whose journal does not
/// fold.
fn mixed() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let runs = [
        ("done-a", Shape::Finished),
        ("parked-a", Shape::Parked),
        ("live-a", Shape::Running),
        ("poisoned", Shape::Refused),
        ("done-b", Shape::Finished),
        ("parked-b", Shape::Parked),
        ("live-b", Shape::Running),
    ];
    folded(&db, |conn| {
        for (minutes, (run_id, shape)) in (0i64..).zip(runs) {
            plant(conn, run_id, shape, &stamp(now() - HOUR - minutes * MINUTE));
        }
    });
    (dir, db)
}

/// Every run a walk of the pages of `brokkr runs --limit 1 <typed>`
/// lists, each page read by the command the page before it printed, as
/// `brokkr` parses it.
fn walked(db: &Path, typed: &[&str]) -> Vec<String> {
    let mut argv: Vec<String> = ["brokkr", "runs", "--limit", "1"]
        .iter()
        .chain(typed)
        .map(ToString::to_string)
        .collect();
    let mut listed = Vec::new();
    for _ in 0..8 {
        let args = parsed(&argv.iter().map(String::as_str).collect::<Vec<_>>()).unwrap();
        let filter = Filter {
            ending: args.ending,
            needs_operator: args.needs_operator,
            hide_settled: args.hide_settled,
        };
        let text = paged(db, filter, (false, args.limit, args.after.as_ref()));
        let rows = lines_of(db, &text).into_iter().filter(|line| {
            !["brokkr · ", "endings", "next page"]
                .iter()
                .any(|head| line.starts_with(head))
        });
        listed.extend(rows.map(|line| line[2..].to_string()));
        let Some(next) = text.lines().last().unwrap().strip_prefix("next page  ") else {
            return listed;
        };
        let [command] = &crate::cli_reference_tests::words(next).unwrap()[..] else {
            panic!("the next page is one command: {next}");
        };
        argv.clone_from(command);
    }
    panic!("no end in eight pages; listed {listed:?}");
}

/// The next page's command repeats every filter the listing was asked
/// for, so a walk of the pages lists exactly the runs the filter keeps,
/// across rows of every group; and where a journal's path is not UTF-8,
/// the words that state the next page name the filter too.
#[test]
fn the_next_page_keeps_every_filter_the_listing_was_asked_for() {
    use std::os::unix::ffi::OsStrExt;
    let (_dir, db) = mixed();
    let cases: [(&[&str], &[&str]); 4] = [
        (&["--ending", "shipped"], &["done-a", "done-b"]),
        (&["--needs-operator"], &["parked-a", "poisoned", "parked-b"]),
        (
            &["--hide-settled"],
            &["parked-a", "live-a", "poisoned", "parked-b", "live-b"],
        ),
        (&["--needs-operator", "--ending", "refused"], &[]),
    ];
    for (typed, kept) in cases {
        assert_eq!(walked(&db, typed), kept, "{typed:?}");
        let mut whole = vec!["brokkr", "runs"];
        whole.extend(typed);
        let args = parsed(&whole).unwrap();
        let filter = Filter {
            ending: args.ending,
            needs_operator: args.needs_operator,
            hide_settled: args.hide_settled,
        };
        let listed: Vec<String> = shape(&db, filter)
            .into_iter()
            .filter(|line| !line.starts_with("brokkr · ") && !line.starts_with("endings"))
            .map(|line| line[2..].to_string())
            .collect();
        assert_eq!(
            listed, kept,
            "{typed:?}: the walk lists what one listing does"
        );
    }
    let bytes = Path::new(std::ffi::OsStr::from_bytes(b"review-\xff.db"));
    let called = Called {
        db: Some(bytes),
        realms: None,
    };
    let every = Filter {
        ending: Some(Ending::DeliveredElsewhere),
        needs_operator: true,
        hide_settled: true,
    };
    let next: brokkr_view::Cursor = "recent:5:done".parse().unwrap();
    assert_eq!(
        super::next_page(1, &next, called, every),
        "next page  --limit 1 --after recent:5:done --ending delivered-elsewhere \
         --needs-operator --hide-settled, with this listing's --db and --realms: a path that \
         is not UTF-8 is not printed as a command"
    );
}
