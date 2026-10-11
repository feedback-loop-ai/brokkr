//! Decision 0081 ruling 3: a spec dialect tracks slices of work, and a
//! slice ends at its archive. A change under `openspec/changes/`, outside
//! `archive/`, whose `tasks.md` has every task ticked is finished, and the
//! dialect's archive step owes it; this file refuses it left in place.
//!
//! The tasks are read as OpenSpec's own parser reads them: a list item
//! (`-` or `*`, behind any blanks, then any blanks or none) that opens on a
//! box, one blank (`[ ]`) open and `[x]` or `[X]` done. A blank is
//! ECMAScript's `\s`, the parser's own: Unicode's white space with a
//! byte-order mark and without U+0085. A list item whose brackets hold more
//! than one character once their blanks are trimmed is a link, not a task.
//! What cannot be read is refused, never skipped: a change with no
//! `tasks.md`, a `tasks.md` with no task, a box that never closes, an empty
//! or widened box (`[]`, `[  ]`, or a mark padded with blanks such as
//! `[ x ]`, which OpenSpec counts as no task at all), a box marked with any
//! other character, and a tracked file under `openspec/changes/` that
//! stands in no change's directory.

use std::collections::BTreeMap;
use std::fmt;

use crate::tracked_files;
use crate::workspace_root::{read, workspace};

/// Where the dialect keeps its changes.
const CHANGES: &str = "openspec/changes/";

/// The directory under [`CHANGES`] the archive step moves a change into.
const ARCHIVE: &str = "archive";

/// The state of one task box.
enum Task {
    Open,
    Done,
}

/// Why a task box cannot be read.
enum Unreadable {
    /// The box opens and never closes.
    Unclosed,
    /// The box is empty, holds more than one blank and nothing else, or pads
    /// its one mark with blanks.
    Widened,
    /// The box holds one character that is neither a blank nor `x`/`X`.
    Marked(String),
}

impl fmt::Display for Unreadable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unclosed => write!(f, "a box that never closes"),
            Self::Widened => write!(f, "an empty or widened box"),
            Self::Marked(mark) => write!(f, "a box marked `{mark}`"),
        }
    }
}

/// Whether `c` is a blank to OpenSpec's parser, ECMAScript's `\s`: Unicode's
/// white space with U+FEFF, a byte-order mark, and without U+0085.
fn blank(c: char) -> bool {
    c == '\u{feff}' || (c.is_whitespace() && c != '\u{85}')
}

/// The task a line holds, `None` when it holds none, or why its box cannot
/// be read.
fn task(line: &str) -> Result<Option<Task>, Unreadable> {
    let Some(rest) = line
        .trim_start_matches(blank)
        .strip_prefix(['-', '*'])
        .and_then(|item| item.trim_start_matches(blank).strip_prefix('['))
    else {
        return Ok(None);
    };
    let Some((inside, _)) = rest.split_once(']') else {
        return Err(Unreadable::Unclosed);
    };
    let mut chars = inside.chars();
    match (chars.next(), chars.next()) {
        (Some(one), None) if blank(one) => Ok(Some(Task::Open)),
        (Some('x' | 'X'), None) => Ok(Some(Task::Done)),
        _ => match inside.trim_matches(blank) {
            mark if mark.chars().count() > 1 => Ok(None),
            mark if mark.is_empty() || mark != inside => Err(Unreadable::Widened),
            mark => Err(Unreadable::Marked(mark.to_string())),
        },
    }
}

/// Why the change `change`, whose `tasks.md` reads `tasks` (`None` when it
/// has none), is finished or cannot be read; empty while its work is open.
fn change_findings(change: &str, tasks: Option<&str>) -> Vec<String> {
    let Some(tasks) = tasks else {
        return vec![format!(
            "{CHANGES}{change}: has no tasks.md, so whether its work is done cannot be read"
        )];
    };
    let (mut open, mut done, mut found) = (0, 0, Vec::new());
    for (at, line) in tasks.lines().enumerate() {
        match task(line) {
            Ok(Some(Task::Open)) => open += 1,
            Ok(Some(Task::Done)) => done += 1,
            Ok(None) => {}
            Err(why) => found.push(format!(
                "{CHANGES}{change}/tasks.md:{}: cannot read the task box: {why}",
                at + 1
            )),
        }
    }
    match (found.is_empty(), open, done) {
        (true, 0, 0) => vec![format!("{CHANGES}{change}/tasks.md: holds no task")],
        (true, 0, done) => vec![format!(
            "{CHANGES}{change}: every task is ticked ({done} of {done}); a finished change \
             is archived by the dialect's archive step (decision 0081)"
        )],
        _ => found,
    }
}

/// Each finding over the `tracked` paths, a `tasks.md` read through
/// `read`: every change outside the archive, and every file under
/// [`CHANGES`] that stands in no change's directory.
fn retention(tracked: &[String], read: impl Fn(&str) -> String) -> Vec<String> {
    let mut found = Vec::new();
    let mut changes: BTreeMap<&str, bool> = BTreeMap::new();
    for path in tracked {
        let Some(within) = path.strip_prefix(CHANGES) else {
            continue;
        };
        match within.split_once('/') {
            Some((ARCHIVE, _)) => {}
            Some((change, file)) => *changes.entry(change).or_default() |= file == "tasks.md",
            None => found.push(format!("{path}: stands in no change's directory")),
        }
    }
    for (change, tasked) in changes {
        let tasks = tasked.then(|| read(&format!("{CHANGES}{change}/tasks.md")));
        found.extend(change_findings(change, tasks.as_deref()));
    }
    found
}

#[test]
fn no_finished_change_is_left_unarchived() {
    let tracked = tracked_files::tracked(&workspace(), &[CHANGES]);
    assert!(
        tracked
            .iter()
            .any(|path| path.starts_with("openspec/changes/archive/")),
        "the walk read none of {CHANGES}"
    );
    assert_eq!(
        retention(&tracked, read),
        Vec::<String>::new(),
        "archive a finished change with the dialect's archive step (decision 0081)"
    );
}

#[test]
fn a_finished_or_unreadable_change_is_refused() {
    let files = BTreeMap::from([
        (
            "openspec/changes/archive/2026-01-01-done/tasks.md",
            "- [x] 1.1 archived\n",
        ),
        (
            "openspec/changes/blank-box/tasks.md",
            "- [x] 1.1 done\n- [\t] 1.2 open in a tabbed box\n- [\u{a0}] 1.3 open in a no-break box\n\
             - [\u{feff}] 1.4 open in a byte-order mark's box\n",
        ),
        (
            "openspec/changes/bom/tasks.md",
            "\u{feff}- [ ] 1.1 open behind a byte-order mark\n- [x] 1.2 done\n",
        ),
        ("openspec/changes/empty/tasks.md", "## 1. Nothing tasked\n"),
        (
            "openspec/changes/finished/tasks.md",
            "- [x] 1.1 one\n* [X] 1.2 two\n    - [x] 1.3 nested\n-\t[x] 1.4 tabbed\n-[x] 1.5\n\
             -\u{2003}[x] 1.6 em-spaced\n\u{feff}-\u{feff}[x] 1.7 byte-order marked\n\
             \u{85}- [ ] behind U+0085, no task to OpenSpec\n",
        ),
        (
            "openspec/changes/no-break/tasks.md",
            "- [x] 1.1 done\n-\u{a0}[ ] 1.2 open behind a no-break space\n",
        ),
        ("openspec/changes/open/proposal.md", ""),
        (
            "openspec/changes/open/tasks.md",
            "## 1. Group\n\n- [x] 1.1 done\n  -  [ ] 1.2 open\n- [LE1 — a link][LE1].\n",
        ),
        ("openspec/changes/stray.md", ""),
        (
            "openspec/changes/unreadable/tasks.md",
            "- [ ] 1.1 open\n- [~] 1.2\n- [] 1.3\n- [  ] 1.4\n- [ 1.5 never closed\n- [ x ] 1.6\n",
        ),
        ("openspec/changes/untasked/proposal.md", ""),
        ("openspec/specs/a/spec.md", ""),
    ]);
    let tracked: Vec<String> = files.keys().map(|path| path.to_string()).collect();
    assert_eq!(
        retention(&tracked, |path| files[path].to_string()),
        [
            "openspec/changes/stray.md: stands in no change's directory",
            "openspec/changes/empty/tasks.md: holds no task",
            "openspec/changes/finished: every task is ticked (7 of 7); a finished change is \
             archived by the dialect's archive step (decision 0081)",
            "openspec/changes/unreadable/tasks.md:2: cannot read the task box: a box marked `~`",
            "openspec/changes/unreadable/tasks.md:3: cannot read the task box: an empty or widened box",
            "openspec/changes/unreadable/tasks.md:4: cannot read the task box: an empty or widened box",
            "openspec/changes/unreadable/tasks.md:5: cannot read the task box: a box that never closes",
            "openspec/changes/unreadable/tasks.md:6: cannot read the task box: an empty or widened box",
            "openspec/changes/untasked: has no tasks.md, so whether its work is done cannot be read",
        ]
    );
}
