//! The steps of one agent definition, each a field or a group of its
//! fields, in the order `parse_agent` reads them: the capability
//! requests, the description and charter, the model chain with its
//! efforts, the hands, the inputs and the results. Every refusal stays in the order it
//! was written, so a file failing several checks reports the same first
//! one.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use brokkr_core::policy::is_identifier;
use serde_json::{Map, Value};

use super::{contained, invalid, name_map, named, string, string_array, LibraryError};

/// The keys an agent definition may write, in the order their steps read.
pub(super) const KEYS: [&str; 11] = [
    "description",
    "charter",
    "models",
    "efforts",
    "tools",
    "capabilities",
    "hands",
    "access",
    "limits",
    "inputs",
    "results",
];

/// The office's capability requests, absent reading as none.
pub(super) fn capabilities(
    map: &Map<String, Value>,
    name: &str,
) -> Result<crate::capabilities::Requests, LibraryError> {
    // Decision 0065 ruling 1: what the office asks for, by abstract name.
    // Syntax only here — whether each name has a definition is the
    // operator's configuration to answer, at lint and at compile.
    let capabilities = match map.get("capabilities") {
        None => Default::default(),
        Some(raw) => crate::capabilities::parse_requests(&format!("agent '{name}'"), raw)
            .map_err(LibraryError::Invalid)?,
    };
    Ok(capabilities)
}

/// What the office says it is: its description, and its charter as
/// written, as contained in the library, and as read.
pub(super) struct Office {
    pub(super) description: String,
    pub(super) charter_rel: String,
    pub(super) charter: PathBuf,
    pub(super) charter_bytes: Vec<u8>,
}

pub(super) fn office(
    root: &Path,
    map: &Map<String, Value>,
    what: &str,
) -> Result<Office, LibraryError> {
    let description = string(map, "description", what)?;
    let charter_rel = string(map, "charter", what)?;
    let charter = contained(root, &charter_rel, what)?;
    let charter_bytes = std::fs::read(&charter)?;
    Ok(Office {
        description,
        charter_rel,
        charter,
        charter_bytes,
    })
}

/// The model chain, and the effort each candidate in it hires.
pub(super) fn chain(
    map: &Map<String, Value>,
    what: &str,
) -> Result<(Vec<String>, BTreeMap<String, String>), LibraryError> {
    let models = string_array(map, "models", what)?;
    if models.is_empty() {
        return invalid(format!(
            "{what} 'models' is empty; an agent names at least one model"
        ));
    }
    named(&models, "models", what)?;
    // A charter names the effort it hires exactly as it names the model
    // (decision 0035 ruling 5), keyed by the candidate it belongs to
    // rather than positionally: a chain reordered in review must not
    // silently re-hire every seat at a different level. Whether a
    // candidate NEEDS one is not knowable here — it depends on the
    // adapter that ends up serving it — so this only refuses an effort
    // named for a candidate that is not in the chain at all.
    let efforts = map.get("efforts").map(|_| name_map(map, "efforts", what));
    let efforts = efforts.transpose()?.unwrap_or_default();
    for candidate in efforts.keys() {
        if !models.contains(candidate) {
            return invalid(format!(
                "{what} 'efforts' names an effort for '{candidate}', which is not \
                 in its 'models' chain [{}]",
                models.join(", ")
            ));
        }
    }
    Ok((models, efforts))
}

/// Decision 0043: one boxed tool instead of a list, refused where a malformed list is.
pub(super) fn hands(
    map: &Map<String, Value>,
    what: &str,
) -> Result<Option<brokkr_protocol::hands::HandsSpec>, LibraryError> {
    let hands = match map.get("hands") {
        None => None,
        Some(raw) => Some(
            brokkr_protocol::hands::HandsSpec::parse(raw)
                .map_err(|problem| LibraryError::Invalid(format!("{what} 'hands': {problem}")))?,
        ),
    };
    Ok(hands)
}

/// A list an office may declare, absent where it names none.
type Declared = Option<Vec<String>>;

/// What the office declares of its verdicts: the inputs it reads and the
/// results it may report, each where it names them.
pub(super) fn declared(
    map: &Map<String, Value>,
    what: &str,
) -> Result<(Declared, Declared), LibraryError> {
    Ok((inputs(map, what)?, results(map, what)?))
}

/// The inputs the office reads, where it names them.
fn inputs(map: &Map<String, Value>, what: &str) -> Result<Option<Vec<String>>, LibraryError> {
    let inputs = match map.get("inputs") {
        None => None,
        Some(_) => Some(string_array(map, "inputs", what)?),
    };
    Ok(inputs)
}

/// The results the office may report, where it names them (decision 0109
/// ruling 11): a non-empty list of result words. A review-panel member
/// is held to it, and a single seat's own `results` lie within it.
fn results(map: &Map<String, Value>, what: &str) -> Result<Option<Vec<String>>, LibraryError> {
    if !map.contains_key("results") {
        return Ok(None);
    }
    let results = string_array(map, "results", what)?;
    if results.is_empty() {
        return invalid(format!(
            "{what} 'results' is empty; an office that declares its results names at least one"
        ));
    }
    match results.iter().find(|word| !is_identifier(word)) {
        Some(word) => invalid(format!(
            "{what} 'results' names '{word}', which does not match ^[a-z0-9][a-z0-9._-]*$"
        )),
        None => Ok(Some(results)),
    }
}
