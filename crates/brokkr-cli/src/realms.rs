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

use brokkr_core::realms::Realm;
use brokkr_runtime::realms::{CrossingReport, World};
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
    /// The files this realm publishes (decision 0057 ruling 1), in map
    /// order. Empty for every realm that draws no crossing, which is
    /// every realm of every world before `forge.realms/v5`.
    pub publishes: Vec<Published>,
    /// The crossings this realm consumes (ruling 2), in map order, each
    /// with the state its pin is in RIGHT NOW.
    pub consumes: Vec<Consumed>,
}

/// One file a realm publishes, as it is read out: the name its consumers
/// know it by, and where in the publishing tree it lives.
pub struct Published {
    pub name: String,
    pub path: String,
}

/// One crossing a realm consumes, as it is read out: the name, the realm
/// that publishes it, and whether the pin is what the map says it is.
pub struct Consumed {
    pub name: String,
    /// The realm that publishes it — never this one (ruling 3.6).
    pub publisher: String,
    pub pin: Pin,
}

/// The three states a consumed pin can be in, and there is no fourth.
/// The same three `brokkr doctor` renders, read off the same
/// [`CrossingReport`] — this readout computes nothing, because a second
/// computation is a second answer waiting to disagree with the one
/// `World::load` already refused or accepted on.
pub enum Pin {
    /// Compared against the publisher's bytes, and they are the pinned
    /// bytes.
    Matching,
    /// Compared, and they are not. Carries the refusal `run` would have
    /// given, read out of the error itself so a moved crossing has
    /// exactly one wording in this build.
    Moved(String),
    /// Never compared to anything, because the publishing realm's own
    /// file could not be read. Not this realm's fault — the publisher's
    /// line carries that — but not a pin that matched either, so it is
    /// never counted among the matching ones (`ca0c765`).
    Unchecked(String),
}

impl Pin {
    /// The one word a script branches on, and the word the frame prints.
    pub fn word(&self) -> &'static str {
        match self {
            Pin::Matching => "matching",
            Pin::Moved(_) => "moved",
            Pin::Unchecked(_) => "unchecked",
        }
    }

    /// Why, for the two states that have a why. `Matching` has none: a
    /// pin that matched is entirely said by the word.
    pub fn detail(&self) -> Option<&str> {
        match self {
            Pin::Matching => None,
            Pin::Moved(detail) | Pin::Unchecked(detail) => Some(detail),
        }
    }
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

/// The widest cell of one column across every crossing line the readout
/// will print, so that two realms' crossings line up under each other the
/// way the realm lines do.
fn crossing_width<'a>(cells: impl Iterator<Item = &'a str>) -> usize {
    cells.map(|cell| Safe::new(cell).width()).max().unwrap_or(0)
}

/// One realm's crossings, indented under its own line: what it publishes,
/// then what it consumes and whether each pin is what the map says it is.
///
/// A realm that draws none contributes nothing at all — not a heading, not
/// a blank line — so a world that never drew a crossing reads out exactly
/// as it did before decision 0057, byte for byte.
fn crossings(row: &Row, name: usize, publisher: usize) -> String {
    let mut out = String::new();
    for published in &row.publishes {
        out.push_str("  publishes  ");
        out.push_str(&Safe::new(&published.name).padded(name));
        out.push_str("  ");
        out.push_str(Safe::new(&published.path).as_str());
        out.push('\n');
    }
    for consumed in &row.consumes {
        out.push_str("  consumes   ");
        out.push_str(&Safe::new(&consumed.name).padded(name));
        out.push_str("  ");
        out.push_str(&Safe::new(&consumed.publisher).padded(publisher));
        out.push_str("  ");
        out.push_str(consumed.pin.word());
        // The detail is the refusal's own words, sanitized like every
        // other string that reaches a terminal from a file somebody else
        // wrote.
        if let Some(detail) = consumed.pin.detail() {
            out.push_str(" · ");
            out.push_str(Safe::new(detail).as_str());
        }
        out.push('\n');
    }
    out
}

/// The world as text: two facts about the world, then one line per
/// realm with its columns aligned, and under each realm the crossings it
/// draws. `hearths` is [`per_realm`]: many hearths are said only where
/// there are many.
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
    let name_width = crossing_width(rows.iter().flat_map(|row| {
        row.publishes
            .iter()
            .map(|published| published.name.as_str())
            .chain(row.consumes.iter().map(|consumed| consumed.name.as_str()))
    }));
    let publisher_width = crossing_width(
        rows.iter()
            .flat_map(|row| row.consumes.iter())
            .map(|consumed| consumed.publisher.as_str()),
    );
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
        out.push_str(&crossings(hearth, name_width, publisher_width));
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
            .map(|row| {
                let mut realm = json!({
                    "name": row.name,
                    "path": row.path,
                    "default_branch": row.branch,
                    "head": row.head,
                    "journal": row.journal,
                });
                // Written only by a realm that has one to write, so a
                // world that never drew a crossing emits the exact bytes
                // it emitted before decision 0057 — the same rule the
                // manifest's `crossings` pin and doctor's lines follow.
                if !row.publishes.is_empty() {
                    realm["publishes"] = json!(row
                        .publishes
                        .iter()
                        .map(|published| json!({
                            "name": published.name,
                            "path": published.path,
                        }))
                        .collect::<Vec<Value>>());
                }
                if !row.consumes.is_empty() {
                    realm["consumes"] = json!(row
                        .consumes
                        .iter()
                        .map(|consumed| json!({
                            "name": consumed.name,
                            "realm": consumed.publisher,
                            // One word, so a script branches on a value
                            // rather than on prose, and the prose that
                            // explains it beside — null where the word
                            // says everything.
                            "pin": consumed.pin.word(),
                            "detail": consumed.pin.detail(),
                        }))
                        .collect::<Vec<Value>>());
                }
                realm
            })
            .collect::<Vec<Value>>(),
    })
}

/// The state one consumed pin is in, read off the report `World::load`
/// already built. Nothing here opens a file, re-derives a digest or
/// compares one: the answer this realm's next run would be given is the
/// answer printed, or the two surfaces would eventually disagree.
///
/// A moved pin is looked up by the fault's direction, the publishing
/// realm and the crossing's name. The three are pure predicates, so the
/// order cannot change the answer; each is asked separately because a
/// realm may consume differently-named crossings from different
/// publishers, and no single field is the answer on its own.
fn pin_of(reports: &[CrossingReport], realm: &str, name: &str, publisher: &str) -> Pin {
    let mine = || reports.iter().filter(|report| report.realm == realm);
    let moved = mine()
        .flat_map(|report| &report.failures)
        .find(|failure| {
            failure.moved() && failure.publisher() == Some(publisher) && failure.crossing() == name
        })
        .map(|failure| Pin::Moved(failure.error().to_string()));
    let unchecked = || {
        mine()
            .flat_map(|report| &report.unchecked)
            .find(|pin| pin.crossing == name && pin.publisher == publisher)
            .map(|pin| Pin::Unchecked(pin.to_string()))
    };
    moved.or_else(unchecked).unwrap_or(Pin::Matching)
}

/// One realm's crossings as they are read out. Both lists come from the
/// MAP — what the realm declared — and only the pin's state comes from
/// the report, so a realm's declarations are shown whole even where one
/// of them could not be checked.
///
/// Shared with `brokkr muninn run`'s dossier (`crate::muninn`), so the
/// two read surfaces render one derivation of the crossings and cannot
/// word a pin two ways. Computing a second answer here — hashing a file,
/// re-running the comparison — would be exactly the disagreement slice
/// (v) removed.
pub(crate) fn crossings_of(
    reports: &[CrossingReport],
    realm: &Realm,
) -> (Vec<Published>, Vec<Consumed>) {
    let publishes = realm
        .published()
        .iter()
        .map(|crossing| Published {
            name: crossing.name.clone(),
            path: crossing.path.clone(),
        })
        .collect();
    let consumes = realm
        .consumed()
        .iter()
        .map(|crossing| Consumed {
            name: crossing.name.clone(),
            publisher: crossing.realm.clone(),
            pin: pin_of(reports, &realm.name, &crossing.name, &crossing.realm),
        })
        .collect();
    (publishes, consumes)
}

/// The rows for a loaded world, each realm's HEAD observed once.
pub fn rows(world: &World) -> Vec<Row> {
    world
        .map
        .realms
        .iter()
        .map(|realm| {
            let (publishes, consumes) = crossings_of(world.crossings_report(), realm);
            Row {
                name: realm.name.clone(),
                path: realm.path.clone(),
                branch: realm.default_branch.clone(),
                head: brokkr_runtime::git_head(&world.path_of(realm))
                    .unwrap_or_else(|| NO_HEAD.to_string()),
                journal: world.journal_of(realm).display().to_string(),
                publishes,
                consumes,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests;
