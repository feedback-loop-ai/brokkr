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
    /// This realm's effective journal (decision 0026 ruling 1): its own
    /// when the map gives it one, else the world's. Read out only when
    /// some realm's differs from the world's — a one-hearth world has
    /// already said its journal once, at the top, and saying it again
    /// per realm would be noise.
    pub journal: String,
}

/// Whether the per-realm journal column is worth printing: some realm's
/// effective journal is not the journal the WORLD itself names.
///
/// Compared against the world's journal rather than against the other
/// realms, because a map whose realms all name one journal other than
/// the world's would otherwise print a header naming a journal no realm
/// reads, and no column to correct it. Compared against the map's own
/// journal rather than against the header, because `--db` renames the
/// header for one invocation without changing what the map says — a v1
/// world read with `--db` grows no column.
pub fn per_realm(world: &World, rows: &[Row]) -> bool {
    let world_journal = world.journal().display().to_string();
    rows.iter().any(|row| row.journal != world_journal)
}

/// The world as text: two facts about the world, then one line per
/// realm with its columns aligned. `hearths` is [`per_realm`]: many
/// hearths are said only where there are many.
pub fn render(source: &str, journal: &str, rows: &[Row], hearths: bool) -> String {
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
    // Many hearths, said only where there are many: a world whose realms
    // all read the journal the world itself names reads exactly as it
    // always did — that journal is named above already — and a world
    // whose realms do not gets told which hearth is whose.
    for (row, hearth) in cells.iter().zip(rows) {
        out.push_str("realm    ");
        for (index, width) in widths.iter().enumerate() {
            out.push_str(&row[index].padded(*width));
            out.push_str("  ");
        }
        out.push_str(row[3].as_str());
        if hearths {
            out.push_str("  ");
            out.push_str(Safe::new(&hearth.journal).as_str());
        }
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
                "journal": row.journal,
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
            journal: world.journal_of(realm).display().to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests;
