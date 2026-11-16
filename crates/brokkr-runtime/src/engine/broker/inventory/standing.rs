//! A standing reservation as admission reads it (decision 0090; decision
//! 0065 slice two, U8a2a; D6, MB3): only what the comparison, the sources'
//! comparison and recovery read of a run, as every build since U8a2a writes
//! it, so a reservation another Brokkr build wrote, whose record this build
//! cannot read whole, still accounts for every writer it reserved.
//!
//! This run's own reservation is read whole, strictly ([`Reservation`]), as
//! every one this build can read whole is; recovery and the join seal only
//! a record read so. Any other is read through the view: its worktree's key,
//! its run, artifact root, journal and settled attempts, and each standing
//! attempt's `protects`, `cleanup`, `serves` and writers, each by its site
//! and its reach — unbounded, or boxed to its writable roots beside the
//! sources its box binds apart. Every other field is ignored. A `protects`
//! or `cleanup` this build does not know is read at its most restrictive:
//! evidence at stake, and no settlement proven. A field the view needs that
//! is missing, a reach of a kind it does not know, or a `format` other than
//! [`FORMAT`] refuses the file by name.

use std::collections::{BTreeMap, BTreeSet};
use std::os::fd::OwnedFd;
use std::path::{Path, PathBuf};

use serde::de::IgnoredAny;
use serde::Deserialize;
use serde_json::Value;

use crate::engine::broker::admission::{
    Protects, Reach, Reservation, Unknown, Witnessed, Writer, RESERVATION,
};
use crate::engine::broker::root::bytes;
use crate::engine::broker::sources::Served;

/// The participation format this build reads: the view's fields as U8a2a
/// first wrote them. A reservation naming no `format` is of it; a build
/// that changes what one of these fields means names a format of its own,
/// which this build refuses by name rather than misread (decision 0090).
const FORMAT: u64 = 1;

/// One run's standing reservation, as admission compares it: its
/// worktree's key, its run, artifact root and journal, each attempt it
/// holds standing, and each it settled; and the record whole, where this
/// build reads it so, which is all recovery may seal again.
#[derive(Debug, Clone, Deserialize)]
pub(in crate::engine::broker) struct Standing {
    pub(in crate::engine::broker) repo: String,
    pub(in crate::engine::broker) run: String,
    pub(in crate::engine::broker) artifacts: PathBuf,
    pub(in crate::engine::broker) journal: PathBuf,
    pub(in crate::engine::broker) attempts: BTreeMap<String, Attempt>,
    pub(in crate::engine::broker) settled: BTreeSet<String>,
    #[serde(skip)]
    pub(in crate::engine::broker) whole: Option<Reservation>,
    /// The format a newer writer stated, where this build read its record
    /// only through the view because it names a field this build does not
    /// know. `None` for a record from before b8e2caff8, whose missing field
    /// is why the whole read failed: that one keeps the view-only route
    /// (R5; decision 0090 ruling 4).
    #[serde(skip)]
    pub(in crate::engine::broker) newer: Option<u64>,
}

/// One standing attempt, as admission compares it: whether it protects
/// evidence, its engine's witness of its cleanup, its writers, and the
/// sources its MCP servers are built from.
#[derive(Debug, Clone, Deserialize)]
pub(in crate::engine::broker) struct Attempt {
    protects: Tolerated<Protects>,
    writers: Vec<Writing>,
    cleanup: Tolerated<Witnessed>,
    #[serde(default)]
    serves: Vec<Served>,
}

/// One site's writer, by its label, and what it can write.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(in crate::engine::broker) struct Writing {
    pub(in crate::engine::broker) site: String,
    pub(in crate::engine::broker) reach: Reaching,
}

/// What a writer can write: a workspace box's writable roots, beside every
/// source it binds apart as authored; or everything its user can, outside
/// one.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(in crate::engine::broker) enum Reaching {
    Boxed {
        writable: Vec<PathBuf>,
        sources: Vec<PathBuf>,
    },
    Unbounded,
}

/// A value of a closed vocabulary as another build may have written it:
/// one this build knows, or one it does not, read at its most restrictive
/// by whatever reads it.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum Tolerated<T> {
    Known(T),
    Unrecognised(IgnoredAny),
}

/// Only the `format` a reservation names, if any.
#[derive(Deserialize)]
struct Format {
    #[serde(default)]
    format: Option<u64>,
}

/// The object field names the whole record this build reads knows, by the
/// path from its root: the `Reservation` itself (`""`), each `Reserved`
/// under `attempts/*`, each `Writer`, `Reach`, `hands::Sealed` with its
/// `Git`, `Source` and `Store`, each `Served` and each `Driver`. A field
/// outside the set its own level holds is one this build does not know,
/// which a newer build stated; decision 0090 ruling 4 lets an additive
/// change or one outside the view name no new format, so the field is the
/// only fact that tells it from a record written before b8e2caff8, whose
/// missing field (not an extra one) fails the whole read (R5).
///
/// A record from before b8e2caff8 sealed `sealed` as a list of sources
/// rather than one object; `sealed/*` is that list's element, so its fields
/// are held to the same names and it stays a view-only record rather than a
/// newer writer's.
const KNOWN: &[(&str, &[&str])] = &[
    (
        "",
        &[
            "repo",
            "run",
            "workdir",
            "artifacts",
            "journal",
            "attempts",
            "settled",
        ],
    ),
    (
        "attempts/*",
        &[
            "protects", "writers", "cleanup", "group", "serves", "drivers",
        ],
    ),
    ("attempts/*/writers/*", &["site", "reach"]),
    ("attempts/*/writers/*/reach", &["boxed", "unbounded"]),
    (
        "attempts/*/writers/*/reach/boxed",
        &["writable", "readable", "sources", "sealed"],
    ),
    (
        "attempts/*/writers/*/reach/boxed/sealed",
        &["home", "worktree", "git", "binds", "store"],
    ),
    (
        "attempts/*/writers/*/reach/boxed/sealed/*",
        &["path", "identity"],
    ),
    (
        "attempts/*/writers/*/reach/boxed/sealed/worktree",
        &["path", "identity"],
    ),
    (
        "attempts/*/writers/*/reach/boxed/sealed/git",
        &["dir", "common"],
    ),
    (
        "attempts/*/writers/*/reach/boxed/sealed/git/dir",
        &["path", "identity"],
    ),
    (
        "attempts/*/writers/*/reach/boxed/sealed/git/common",
        &["path", "identity"],
    ),
    (
        "attempts/*/writers/*/reach/boxed/sealed/binds/*",
        &["path", "identity"],
    ),
    (
        "attempts/*/writers/*/reach/boxed/sealed/store",
        &["place", "spelling"],
    ),
    ("attempts/*/serves/*", &["source", "entries", "link"]),
    (
        "attempts/*/drivers/*",
        &["site", "begun", "cleanup", "group"],
    ),
];

/// Whether `value` names any object field its own level does not hold, or an
/// object where the record holds none. The `attempts` map's keys are attempt
/// ids, not fields, so only its values are judged.
fn names_unknown(value: &Value) -> bool {
    holds_unknown(value, "")
}

/// [`names_unknown`] at the schema path `path`, the level the object there
/// must hold.
fn holds_unknown(value: &Value, path: &str) -> bool {
    match value {
        Value::Object(fields) => {
            if path == "attempts" {
                return fields
                    .values()
                    .any(|value| holds_unknown(value, "attempts/*"));
            }
            let Some((_, known)) = KNOWN.iter().find(|(at, _)| *at == path) else {
                // An object where the record holds none: fields this build
                // does not know at this level.
                return true;
            };
            fields.iter().any(|(key, value)| {
                !known.contains(&key.as_str()) || holds_unknown(value, &child(path, key))
            })
        }
        Value::Array(items) => items
            .iter()
            .any(|item| holds_unknown(item, &format!("{path}/*"))),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => false,
    }
}

/// The schema path of the field `key` below `path`.
fn child(path: &str, key: &str) -> String {
    if path.is_empty() {
        key.to_string()
    } else {
        format!("{path}/{key}")
    }
}

impl Attempt {
    /// Whether it protects evidence: one this build does not know does.
    fn protects(&self) -> bool {
        match self.protects {
            Tolerated::Known(Protects::Evidence) | Tolerated::Unrecognised(_) => true,
            Tolerated::Known(Protects::Nothing) => false,
        }
    }

    /// Whether its engine sealed its positive settlement fact (M1): never
    /// one this build does not know. The release's proof reads it through
    /// the view of another build's record (decision 0108 ruling 3).
    pub(in crate::engine::broker) fn proved(&self) -> bool {
        match self.cleanup {
            Tolerated::Known(Witnessed::Settled) => true,
            Tolerated::Known(Witnessed::Settling | Witnessed::Unresolved)
            | Tolerated::Unrecognised(_) => false,
        }
    }
}

impl Standing {
    /// Every writer its standing attempts reserved.
    pub(in crate::engine::broker) fn writers(&self) -> impl Iterator<Item = &Writing> {
        self.attempts.values().flat_map(|attempt| &attempt.writers)
    }

    /// Whether any of its standing attempts protects evidence.
    pub(in crate::engine::broker) fn protects(&self) -> bool {
        self.attempts.values().any(Attempt::protects)
    }

    /// Every MCP server source its standing attempts reserved (U8a2b).
    pub(in crate::engine::broker) fn serves(&self) -> impl Iterator<Item = &Served> {
        self.attempts.values().flat_map(|attempt| &attempt.serves)
    }

    /// Each standing attempt whose engine sealed its positive settlement
    /// fact (M1), by its id.
    pub(in crate::engine::broker) fn proved(&self) -> impl Iterator<Item = &String> {
        let proved = self.attempts.iter().filter(|(_, attempt)| attempt.proved());
        proved.map(|(id, _)| id)
    }

    /// The reservation read only through the view, each attempt of
    /// `proven` settled for this admission alone: never sealed, so the
    /// build that wrote it reads it as it wrote it (decision 0090).
    pub(in crate::engine::broker) fn released(mut self, proven: &[String]) -> Standing {
        for attempt in proven {
            self.attempts.remove(attempt);
            self.settled.insert(attempt.clone());
        }
        self
    }
}

impl From<Reservation> for Standing {
    fn from(whole: Reservation) -> Standing {
        let attempts = whole.attempts.iter().map(|(id, reserved)| {
            let attempt = Attempt {
                protects: Tolerated::Known(reserved.protects),
                writers: reserved.writers.iter().map(Writing::from).collect(),
                cleanup: Tolerated::Known(reserved.cleanup),
                serves: reserved.serves.clone(),
            };
            (id.clone(), attempt)
        });
        Standing {
            repo: whole.repo.clone(),
            run: whole.run.clone(),
            artifacts: whole.artifacts.clone(),
            journal: whole.journal.clone(),
            attempts: attempts.collect(),
            settled: whole.settled.clone(),
            whole: Some(whole),
            newer: None,
        }
    }
}

impl From<&Writer> for Writing {
    fn from(writer: &Writer) -> Writing {
        let reach = match &writer.reach {
            Reach::Boxed {
                writable, sources, ..
            } => Reaching::Boxed {
                writable: writable.clone(),
                sources: sources.clone(),
            },
            Reach::Unbounded => Reaching::Unbounded,
        };
        Writing {
            site: writer.site.clone(),
            reach,
        }
    }
}

/// The reservation in the run's directory `dir` at `at`: whole where this
/// build reads it so; else, where it is not this run's own (`own`), through
/// the view, in a format the view reads. Otherwise an unknown participant
/// there, or one written in a format this build does not read. A record the
/// whole read refused only because it names a field this build does not
/// know is a newer writer's, and carries the format it stated (or this
/// build's own) for the format advice, never the re-provisioning route
/// (R5).
pub(in crate::engine::broker) fn standing(
    dir: &OwnedFd,
    at: &Path,
    own: bool,
) -> Result<Standing, Unknown> {
    let path = at.join(RESERVATION);
    let refused = || Unknown::Participant(path.clone());
    let bytes = bytes(dir, RESERVATION, at)?;
    if let Ok(whole) = serde_json::from_slice::<Reservation>(&bytes) {
        return Ok(Standing::from(whole));
    }
    (!own).then_some(()).ok_or_else(refused)?;
    let stated: Value = serde_json::from_slice(&bytes).or(Err(refused()))?;
    let Format { format } = serde_json::from_value(stated.clone()).or(Err(refused()))?;
    if let Some(format) = format.filter(|format| *format != FORMAT) {
        return Err(Unknown::Formatted(path, format));
    }
    let mut view: Standing = serde_json::from_value(stated.clone()).or(Err(refused()))?;
    view.newer = names_unknown(&stated).then_some(format.unwrap_or(FORMAT));
    Ok(view)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// A reservation with every optional field set, as this build writes it:
    /// one attempt, one boxed writer, one sealed reach with its Git
    /// directories, binds and secrets store, one served input and one
    /// driver, each group written.
    fn full() -> Value {
        json!({
            "repo": "repo",
            "run": "run-a",
            "workdir": "/work",
            "artifacts": "/artifacts",
            "journal": "/journal",
            "attempts": {
                "a1": {
                    "protects": "nothing",
                    "writers": [{
                        "site": "work",
                        "reach": {
                            "boxed": {
                                "writable": ["/work"],
                                "readable": ["/read"],
                                "sources": ["/work"],
                                "sealed": {
                                    "home": "/home",
                                    "worktree": {"path": "/work", "identity": [1, 2]},
                                    "git": {
                                        "dir": {"path": "/work/.git", "identity": [1, 2]},
                                        "common": {"path": "/work/.git", "identity": [1, 2]}
                                    },
                                    "binds": [{"path": "/bind", "identity": [1, 2]}],
                                    "store": {"place": "/store", "spelling": "/store"}
                                }
                            }
                        }
                    }],
                    "cleanup": "settling",
                    "group": 7,
                    "serves": [{"source": "/served"}],
                    "drivers": [{
                        "site": "work",
                        "begun": [1, "head"],
                        "cleanup": "settling",
                        "group": 7
                    }]
                }
            },
            "settled": ["a0"]
        })
    }

    /// F2 (0071 rulings 5 and 3): every field name this build writes,
    /// serialised from the record's own type with every optional field set,
    /// is one the level that holds it knows.
    #[test]
    fn a_reservation_with_every_optional_field_is_no_newer_writers_record() {
        let written: Reservation = serde_json::from_value(full()).unwrap();
        let serialised = serde_json::to_value(&written).unwrap();
        assert!(
            !names_unknown(&serialised),
            "every field this build writes is known at its own level"
        );
    }

    /// F2: a word this build knows at a level that does not hold it is a
    /// newer writer's field, not a known one. The flat list read it as known
    /// and routed the record to the re-provisioning advice.
    #[test]
    fn a_known_word_at_a_level_that_does_not_hold_it_is_a_newer_writers_record() {
        let mut wrong = full();
        wrong["attempts"]["a1"]["repo"] = json!("a newer writer's field");
        assert!(
            names_unknown(&wrong),
            "a known word at the wrong level is a newer writer's"
        );
        let mut later = full();
        later["attempts"]["a1"]["a_field_from_a_later_build"] = json!(1);
        assert!(
            names_unknown(&later),
            "an unknown field is a newer writer's"
        );
    }

    /// #1099 R1(c): an object where this build's record holds a plain value
    /// is a newer writer's shape, whatever fields it carries. Removal
    /// control: answer `false` for a path no level names, and this fails.
    #[test]
    fn an_object_where_the_record_holds_none_is_a_newer_writers_record() {
        let mut shaped = full();
        shaped["attempts"]["a1"]["cleanup"] = json!({"x": 1});
        assert!(
            names_unknown(&shaped),
            "an object at a level that holds none is a newer writer's"
        );
    }

    /// #1099 R1(a): a record from before b8e2caff8 sealed `sealed` as a list
    /// of `{path, identity}` sources, and stays a view-only record, no newer
    /// writer's; a field beside them in an element is one. Removal control:
    /// drop the `sealed/*` row from `KNOWN`, or either of its words, and this
    /// fails.
    #[test]
    fn a_sealed_list_from_before_b8e2caff8_is_no_newer_writers_record() {
        let mut older = full();
        let sealed = json!([{"path": "/work", "identity": [1, 2]}]);
        older["attempts"]["a1"]["writers"][0]["reach"]["boxed"]["sealed"] = sealed;
        assert!(
            !names_unknown(&older),
            "a list of sources sealed is this build's view-only record"
        );
        let mut later = older.clone();
        later["attempts"]["a1"]["writers"][0]["reach"]["boxed"]["sealed"][0]["a_field"] = json!(1);
        assert!(
            names_unknown(&later),
            "an unknown field in an element is a newer writer's"
        );
    }

    /// #1099 R1(b): each kind of served input this build writes — a source,
    /// the entries of a directory, a link — serialised from the record's own
    /// type is a field the level that holds it knows. Removal control: drop
    /// `entries` or `link` from the `serves/*` row of `KNOWN`, and this
    /// fails.
    #[test]
    fn a_served_input_of_each_kind_is_no_newer_writers_record() {
        let mut served = full();
        served["attempts"]["a1"]["serves"] =
            json!([{"source": "/s"}, {"entries": "/e"}, {"link": "/l"}]);
        let written: Reservation = serde_json::from_value(served).unwrap();
        let serialised = serde_json::to_value(&written).unwrap();
        assert!(
            !names_unknown(&serialised),
            "every served kind this build writes is known at its own level"
        );
    }

    /// #1099 R1(d): an unknown field inside a list's item is read at the
    /// item's own level, for the writers and for the drivers alike.
    #[test]
    fn an_unknown_field_inside_a_listed_item_is_a_newer_writers_record() {
        for list in ["writers", "drivers"] {
            let mut later = full();
            later["attempts"]["a1"][list][0]["a_field_from_a_later_build"] = json!(1);
            assert!(
                names_unknown(&later),
                "an unknown field in {list}[0] is a newer writer's"
            );
        }
    }
}
