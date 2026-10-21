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

impl Attempt {
    /// Whether it protects evidence: one this build does not know does.
    fn protects(&self) -> bool {
        match self.protects {
            Tolerated::Known(Protects::Evidence) | Tolerated::Unrecognised(_) => true,
            Tolerated::Known(Protects::Nothing) => false,
        }
    }

    /// Whether its engine sealed its positive settlement fact (M1): never
    /// one this build does not know.
    fn proved(&self) -> bool {
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
/// there, or one written in a format this build does not read.
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
    let Format { format } = serde_json::from_slice(&bytes).or(Err(refused()))?;
    if let Some(format) = format.filter(|format| *format != FORMAT) {
        return Err(Unknown::Formatted(path, format));
    }
    serde_json::from_slice(&bytes).or(Err(refused()))
}
