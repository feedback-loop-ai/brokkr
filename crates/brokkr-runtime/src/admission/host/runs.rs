//! Decision 0110 ruling 7, amending decision 0068 ruling 3:
//! `forge.host/v4` requires a `runs` object, `{"ceiling": n}` with n at
//! least 1, the most runs that may read `running` in the journal a pass
//! judges. Its shape, closed with a ceiling of at least one, is the strict
//! reader's, as `boxed_builds` is. Which version may write it is judged
//! here: a v4 file that leaves it out, and a v1, v2 or v3 file that writes
//! it, each refuse by their own line. Every v1, v2 and v3 file that leaves
//! it out still reads, and declares no run ceiling.

use std::num::NonZeroU32;
use std::path::Path;

use thiserror::Error;

use super::{read as read_host, Ceiling, Hosting, Schema};

/// Why a file's run ceiling is not one the loader can obey.
#[derive(Debug, Error)]
pub enum Problem {
    /// A v4 file that declares no `runs`.
    #[error("forge.host/v4 declares runs.ceiling, and the file writes no runs")]
    Missing,
    /// A `runs` object in a file of an earlier version.
    #[error("{0} declares no runs; only forge.host/v4 does")]
    NotV4(&'static str),
}

impl Problem {
    /// The contract the refusal names: v4 for a v4 file without its
    /// ceiling, and the file's own version for one that writes it early.
    pub(super) fn contract(&self) -> &'static str {
        match self {
            Problem::Missing => Schema::V4.contract(),
            Problem::NotV4(declared) => declared,
        }
    }
}

impl Schema {
    /// The version's name, as the file writes it.
    pub(super) fn contract(self) -> &'static str {
        match self {
            Schema::V1 => "forge.host/v1",
            Schema::V2 => "forge.host/v2",
            Schema::V3 => "forge.host/v3",
            Schema::V4 => "forge.host/v4",
        }
    }
}

/// The run ceiling a file of `schema` declares in `written`: v4's, which
/// it must write, and none for an earlier version, which must not.
pub(super) fn read(
    schema: Schema,
    written: Option<Ceiling>,
) -> Result<Option<NonZeroU32>, Problem> {
    match (schema, written) {
        (Schema::V4, Some(runs)) => Ok(Some(runs.ceiling)),
        (Schema::V4, None) => Err(Problem::Missing),
        (Schema::V1 | Schema::V2 | Schema::V3, None) => Ok(None),
        (older @ (Schema::V1 | Schema::V2 | Schema::V3), Some(_)) => {
            Err(Problem::NotV4(older.contract()))
        }
    }
}

/// The version of the host file at `file` where it reads and declares no
/// run ceiling, for `brokkr doctor` (decision 0110 ruling 7): under it,
/// every entry the capacity checks would judge waits as `runs_undeclared`.
/// `None` for a file that declares one, and for one absent, unreadable or
/// invalid, which other lines name.
pub fn runs_undeclared(file: &Path) -> Option<&'static str> {
    match read_host(file) {
        Ok(Hosting::Declared(config)) if config.runs.is_none() => Some(config.schema.contract()),
        Ok(Hosting::Declared(_) | Hosting::Absent | Hosting::Unreadable(_)) | Err(_) => None,
    }
}

#[cfg(test)]
mod tests;
