//! Validation of the frozen seat-record contract — v1 (decision 0034)
//! and v2 (decision 0035) through v6 (decision 0065 ruling 8), never one
//! pretending to be another.
//!
//! Every schema is embedded beside this module so a packaged `brokkr`
//! remains an offline verifier. A test pins each embedded copy to the
//! published file in `contracts/`; they cannot drift inside this tree.
//!
//! **A record is validated against the version its run's engine wrote**
//! (decision 0035 ruling 7). There is no per-record version marker and
//! the journal is append-only, so the discriminator has to be a fact the
//! run already carries: the `engine` string in the `run/started`
//! manifest, which both manifest lineages keep. An engine older than the
//! one v2 landed in wrote v1 records and is read under v1 — which is
//! what keeps decision 0034 ruling 5's "old journals remain readable"
//! true, and what makes a v2-only field on such a record a refusal
//! rather than a quiet admission.
//!
//! This module is the version table and its dispatch; the checks that
//! judge a record or a journal against the chosen version are
//! [`validation`]'s, re-exported here at their old paths.

use std::sync::OnceLock;

use thiserror::Error;

mod validation;

pub use validation::validate_seat_record;
pub(crate) use validation::{record_of, validate_events};

const SCHEMA_V1: &str = include_str!("seat-record.v1.schema.json");
const SCHEMA_V2: &str = include_str!("seat-record.v2.schema.json");
const SCHEMA_V3: &str = include_str!("seat-record.v3.schema.json");
const SCHEMA_V4: &str = include_str!("seat-record.v4.schema.json");
const SCHEMA_V5: &str = include_str!("seat-record.v5.schema.json");
const SCHEMA_V6: &str = include_str!("seat-record.v6.schema.json");

const CONTRACT_V1: &str = "contracts/seat-record.v1.schema.json";
const CONTRACT_V2: &str = "contracts/seat-record.v2.schema.json";
const CONTRACT_V3: &str = "contracts/seat-record.v3.schema.json";
const CONTRACT_V4: &str = "contracts/seat-record.v4.schema.json";
const CONTRACT_V5: &str = "contracts/seat-record.v5.schema.json";
const CONTRACT_V6: &str = "contracts/seat-record.v6.schema.json";

/// The engine line in which seat-record v2 landed. A run whose
/// `run/started` manifest names an older engine is read under v1.
///
/// The manifest's `engine` is the crate version and carries no position
/// WITHIN a line, so the boundary is the line rather than a tag: v2
/// landed inside 0.8.0's development line, after the 0.8.0 tag, and a
/// constant naming a future release would misfile every record this
/// engine writes before that release ships. Reading a tagged-0.8.0
/// journal under v2 refuses nothing it wrote — v2 adds optional
/// properties and takes none away, so every valid v1 record is a valid
/// v2 record — while every engine before that line is dispatched to v1
/// exactly.
///
/// Seat-record v3 (decision 0034 ruling 7) landed in this same line, and
/// so shares this boundary. `engine` carries no position WITHIN a line,
/// so v2 and v3 cannot be told apart by it — both landed after the 0.8.0
/// tag. The newest contract in a line therefore wins, on exactly the
/// argument above: v3 adds an optional property and takes none away, so
/// every valid v2 record is a valid v3 record. The consequence is that
/// no engine string selects v2; it stays published, pinned and directly
/// nameable, but dispatch never chooses it.
const V2_ENGINE: (u64, u64, u64) = (0, 8, 0);

/// The engine line in which seat-record v4 landed (decision 0046 ruling
/// 3, with the commission's erratum: v4, additive on v3). Drawn at the
/// 0.9 line on the same argument as `V2_ENGINE`: `engine` carries no
/// position within a line, v4 adds one optional property and takes none
/// away, so a journal the tagged 0.9.0 or 0.9.1 engine wrote — which
/// carries no `boundary` — validates under v4 exactly as it did under
/// v3, and every engine before the line is dispatched to v3 exactly.
const V4_ENGINE: (u64, u64, u64) = (0, 9, 0);

/// The engine line in which seat-record v5 landed (proposed decision
/// 0056 ruling 7, under the `boundary-record` requirement's amended
/// dispatch). Drawn at the 0.10 line on the same argument as `V2_ENGINE`
/// and `V4_ENGINE`: `engine` carries no position within a line, v5 adds
/// only optional properties and widens only existing enums, so every
/// record the shipped 0.10.0 engine already wrote — the codex launch row
/// carrying `launch: resumed` with no root among them — validates under
/// v5 exactly as it did under v4, and every engine before the line is
/// dispatched to v4 exactly.
///
/// The two conditions v5 adds are scoped on `site_ref`, the one
/// within-row fact only an engine enacting 0056 writes, precisely so
/// that widening this boundary to cover already-written 0.10.0 rows
/// refuses none of them.
const V5_ENGINE: (u64, u64, u64) = (0, 10, 0);

/// The engine line in which seat-record v6 landed (decision 0065 ruling
/// 8, the slice-two CC3/SC4 contract). Drawn at the 0.12 line, the
/// development line on main after the 0.12.0 tag, on the same argument
/// as `V2_ENGINE`: `engine` carries no position within a line, and v6
/// adds only optional properties, keeps v5's 80-byte bound and turn
/// dependency on every unattributed row, and scopes both widenings on
/// the attribution group no earlier engine wrote. So every record the
/// tagged 0.12.0 engine already wrote validates under v6 exactly as it
/// did under v5, and the 0.10 and 0.11 lines are dispatched to v5.
const V6_ENGINE: (u64, u64, u64) = (0, 12, 0);

/// Which seat-record contract a record is judged against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeatRecordVersion {
    V1,
    V2,
    V3,
    V4,
    V5,
    V6,
}

impl SeatRecordVersion {
    /// The contract the named engine wrote. An engine string that does
    /// not parse as `major.minor.patch` is treated as older than the
    /// boundary: a version this binary cannot read is not a licence to
    /// admit fields its writer could not have produced.
    pub fn of_engine(engine: &str) -> SeatRecordVersion {
        // Three arms, not four, and the missing one is the point: v2 and
        // v3 share the 0.8.0 line (see `V2_ENGINE`), so no engine string
        // can select v2 — the newest contract in a line always wins. v2
        // stays a published, pinned contract and a version a caller can
        // name directly to judge a record against it; it is simply never
        // what dispatch chooses. `V2_ENGINE` still draws v1's boundary,
        // `V4_ENGINE` draws v3's (decision 0046), `V5_ENGINE` draws v4's
        // (proposed decision 0056 ruling 7), and `V6_ENGINE` draws v5's
        // (decision 0065 ruling 8).
        match semver_triple(engine) {
            Some(version) if version >= V6_ENGINE => SeatRecordVersion::V6,
            Some(version) if version >= V5_ENGINE => SeatRecordVersion::V5,
            Some(version) if version >= V4_ENGINE => SeatRecordVersion::V4,
            Some(version) if version >= V2_ENGINE => SeatRecordVersion::V3,
            _ => SeatRecordVersion::V1,
        }
    }

    /// The contract a run writes under, from the `engine` its manifest
    /// names. The append fence and the export and verify sweeps all ask
    /// this, so they cannot read one run two ways. A manifest that names
    /// no engine is read under v1: the older contract is the safe
    /// reading, because it admits strictly less.
    pub(crate) fn of_manifest(engine: Option<&str>) -> SeatRecordVersion {
        engine.map_or(SeatRecordVersion::V1, SeatRecordVersion::of_engine)
    }

    /// The published file this version validates against, named in the
    /// refusal so a reader knows which contract was applied.
    pub fn contract(self) -> &'static str {
        match self {
            SeatRecordVersion::V1 => CONTRACT_V1,
            SeatRecordVersion::V2 => CONTRACT_V2,
            SeatRecordVersion::V3 => CONTRACT_V3,
            SeatRecordVersion::V4 => CONTRACT_V4,
            SeatRecordVersion::V5 => CONTRACT_V5,
            SeatRecordVersion::V6 => CONTRACT_V6,
        }
    }

    fn source(self) -> &'static str {
        match self {
            SeatRecordVersion::V1 => SCHEMA_V1,
            SeatRecordVersion::V2 => SCHEMA_V2,
            SeatRecordVersion::V3 => SCHEMA_V3,
            SeatRecordVersion::V4 => SCHEMA_V4,
            SeatRecordVersion::V5 => SCHEMA_V5,
            SeatRecordVersion::V6 => SCHEMA_V6,
        }
    }

    /// Where this version's compiled validator is kept, compiled once on
    /// first use by [`validation`]. A version's bytes, contract and cache
    /// are its row of this table, so a new version is added here alone.
    fn compiled(self) -> &'static OnceLock<jsonschema::Validator> {
        match self {
            SeatRecordVersion::V1 => &VALIDATOR_V1,
            SeatRecordVersion::V2 => &VALIDATOR_V2,
            SeatRecordVersion::V3 => &VALIDATOR_V3,
            SeatRecordVersion::V4 => &VALIDATOR_V4,
            SeatRecordVersion::V5 => &VALIDATOR_V5,
            SeatRecordVersion::V6 => &VALIDATOR_V6,
        }
    }
}

/// `major.minor.patch`, ignoring any pre-release or build suffix.
fn semver_triple(version: &str) -> Option<(u64, u64, u64)> {
    let core = version
        .split(['-', '+'])
        .next()
        .filter(|core| !core.is_empty())?;
    let mut parts = core.split('.');
    let mut next = || parts.next()?.parse::<u64>().ok();
    let triple = (next()?, next()?, next()?);
    match parts.next() {
        Some(_) => None,
        None => Some(triple),
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
#[error("seat record at journal seq {seq} violates {contract} at {path}")]
pub struct SeatRecordError {
    pub seq: u64,
    pub path: String,
    /// The published contract the record was judged against.
    pub contract: &'static str,
}

static VALIDATOR_V1: OnceLock<jsonschema::Validator> = OnceLock::new();
static VALIDATOR_V2: OnceLock<jsonschema::Validator> = OnceLock::new();
static VALIDATOR_V3: OnceLock<jsonschema::Validator> = OnceLock::new();
static VALIDATOR_V4: OnceLock<jsonschema::Validator> = OnceLock::new();
static VALIDATOR_V5: OnceLock<jsonschema::Validator> = OnceLock::new();
static VALIDATOR_V6: OnceLock<jsonschema::Validator> = OnceLock::new();

#[cfg(test)]
mod tests;
