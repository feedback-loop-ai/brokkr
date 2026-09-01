//! `brokkr realms` — the world, read out (decision 0023 ruling 6).
//!
//! A read surface like every other: it opens no journal, starts no run,
//! writes nothing. It states the map it read, the journal that world
//! writes, and one line per realm — name, path, default branch, and the
//! HEAD observed in that tree right now.
//!
//! Plain, by 0019 law 4 and ruling 10's summon/observe law: the file is
//! `realms.json`, the verb is `realms`, and the tree the lore names
//! appears nowhere in the machine's mouth.

use brokkr_runtime::realms::World;
use serde_json::{json, Value};

use crate::render::Safe;

/// A realm whose tree has no readable HEAD — no git repository there, or
/// no commit yet. Absent, said plainly, the way every other readout
/// marks an absent fact.
pub const NO_HEAD: &str = "-";

/// One realm as it is read out. Built here so the rendering below stays
/// pure: the git reads happen once, in [`list`].
pub struct Row {
    pub name: String,
    pub path: String,
    pub branch: String,
    pub head: String,
}

/// The world as text: two facts about the world, then one line per
/// realm with its columns aligned.
pub fn render(source: &str, journal: &str, rows: &[Row]) -> String {
    let cells: Vec<[Safe; 4]> = rows
        .iter()
        .map(|row| {
            [
                Safe::new(&row.name),
                Safe::new(&row.path),
                Safe::new(&row.branch),
                Safe::new(&row.head),
            ]
        })
        .collect();
    let mut widths = [0usize; 3];
    for row in &cells {
        for (index, width) in widths.iter_mut().enumerate() {
            *width = (*width).max(row[index].width());
        }
    }
    let mut out = format!(
        "map      {}\njournal  {}\n",
        Safe::new(source).as_str(),
        Safe::new(journal).as_str()
    );
    for row in &cells {
        out.push_str("realm    ");
        for (index, width) in widths.iter().enumerate() {
            out.push_str(&row[index].padded(*width));
            out.push_str("  ");
        }
        out.push_str(row[3].as_str());
        out.push('\n');
    }
    out
}

/// The same world as a value, for `--json`. Derived from the SAME rows
/// the text renders, so the two surfaces can never disagree about what
/// the world is — only about how it is spelled. Unescaped, because a
/// consumer parsing JSON is not a terminal: escaping is the text
/// surface's job, and doing it here would corrupt the data.
pub fn view(source: &str, journal: &str, rows: &[Row]) -> Value {
    json!({
        "map": source,
        "journal": journal,
        "realms": rows
            .iter()
            .map(|row| json!({
                "name": row.name,
                "path": row.path,
                "default_branch": row.branch,
                "head": row.head,
            }))
            .collect::<Vec<Value>>(),
    })
}

/// The rows for a loaded world, each realm's HEAD observed once.
pub fn rows(world: &World) -> Vec<Row> {
    world
        .map
        .realms
        .iter()
        .map(|realm| Row {
            name: realm.name.clone(),
            path: realm.path.clone(),
            branch: realm.default_branch.clone(),
            head: brokkr_runtime::git_head(&world.path_of(realm))
                .unwrap_or_else(|| NO_HEAD.to_string()),
        })
        .collect()
}

#[cfg(test)]
mod tests;
