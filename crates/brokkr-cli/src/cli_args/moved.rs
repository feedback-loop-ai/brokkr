//! The flat spellings 0.16 still parses (decision 0113 ruling 4). A root
//! that moved under a family is respelled before the tree parses it, so
//! the old spelling reaches the same verb with the same arguments, is in
//! no `--help`, and says its new spelling once on stderr. The aliases go
//! in 0.17.

use std::ffi::OsString;

/// Each root that moved, by its old name, and the path it moved to. A
/// root that did not move (the daily verbs, `runs`, `queue` and the
/// engine's hidden verbs) is not here, and is never respelled.
const MOVED: [(&str, &[&str]); 30] = [
    ("inspect", &["runs", "inspect"]),
    ("seats", &["runs", "seats"]),
    ("transcript", &["runs", "transcript"]),
    ("compare", &["runs", "compare"]),
    ("replay", &["runs", "replay"]),
    ("rerun", &["runs", "rerun"]),
    ("conclude", &["runs", "conclude"]),
    ("operator", &["runs", "operator"]),
    ("muninn", &["runs", "muninn"]),
    ("export", &["journal", "export"]),
    ("import", &["journal", "import"]),
    ("verify-run", &["journal", "verify"]),
    ("backup", &["journal", "backup"]),
    ("restore", &["journal", "restore"]),
    ("anchor", &["journal", "anchor"]),
    ("keep-refs", &["journal", "keep-refs"]),
    ("evidence", &["journal", "evidence"]),
    ("recipes", &["library", "recipes"]),
    ("agents", &["library", "agents"]),
    ("compile", &["library", "compile"]),
    ("dispatch", &["queue", "dispatch"]),
    ("sweep", &["queue", "sweep"]),
    ("analytics", &["stats", "analytics"]),
    ("graph", &["stats", "graph"]),
    ("costs", &["stats", "costs"]),
    ("ledger", &["stats", "ledger"]),
    ("realms", &["system", "realms"]),
    ("secrets", &["system", "secrets"]),
    ("probe", &["system", "probe"]),
    ("ui", &["system", "ui"]),
];

/// An old flat spelling that was typed, and where its verb lives now.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Moved {
    old: &'static str,
    new: &'static [&'static str],
}

impl std::fmt::Display for Moved {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "brokkr: '{}' is now 'brokkr {}'; the old spelling goes in 0.17",
            self.old,
            self.new.join(" ")
        )
    }
}

/// The command line as the tree parses it: an old flat spelling in the
/// verb's place is replaced by its new path, and named, and every other
/// argument stays exactly as it was typed.
pub(crate) fn respelled<T: Into<OsString>>(
    argv: impl IntoIterator<Item = T>,
) -> (Vec<OsString>, Option<Moved>) {
    let mut argv: Vec<OsString> = argv.into_iter().map(Into::into).collect();
    let typed = argv.get(1).and_then(|verb| verb.to_str());
    let Some(&(old, new)) = MOVED.iter().find(|(old, _)| typed == Some(*old)) else {
        return (argv, None);
    };
    argv.splice(1..2, new.iter().map(OsString::from));
    (argv, Some(Moved { old, new }))
}

#[cfg(test)]
mod tests;
