//! Why a dialect is not usable (decision 0042): one variant per check, so
//! a test can name the check that fired even where two checks share their
//! words. Its `Display` is the operator's text byte for byte; [`refused`]
//! folds it into [`DialectError::Invalid`], the one place it becomes text
//! (#353).

use super::{DialectError, SCHEMA_V1, SCHEMA_V2, SCHEMA_V3};

/// The one sentence every identity field's emptiness refuses with.
const NON_EMPTY: &str = "name, tool binary, measured version and install package must be non-empty";

/// Why a dialect was refused, each field borrowed from the dialect or the
/// surface reading it.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub(super) enum DialectRefusal<'a> {
    /// The bytes call themselves a version this surface does not read.
    #[error("it calls itself '{schema}'; this reads {} ({note})", .accepted.join(", "))]
    Version {
        schema: &'a str,
        accepted: &'a [&'a str],
        note: &'a str,
    },
    /// A v1 carries the install that landed in v2.
    #[error(
        "a '{}' dialect carries no tool install; \
         the install identity landed in {}",
        SCHEMA_V1,
        SCHEMA_V2
    )]
    InstallTooEarly,
    /// A v2 or v3 names nothing that installs its tool.
    #[error("a '{schema}' dialect names what installs its tool")]
    Uninstalled { schema: &'a str },
    /// An archive below v3 carries the instruction that landed in v3.
    #[error(
        "a '{schema}' archive carries no instruction; \
         the archive instruction landed in {}",
        SCHEMA_V3
    )]
    InstructionTooEarly { schema: &'a str },
    /// A v3 archive command carries no instruction.
    #[error("a '{}' archive command names the instruction it carries", SCHEMA_V3)]
    Uninstructed,
    /// The dialect's name trims to empty.
    #[error("{}", NON_EMPTY)]
    EmptyName,
    /// The tool binary trims to empty.
    #[error("{}", NON_EMPTY)]
    EmptyBinary,
    /// The measured version trims to empty.
    #[error("{}", NON_EMPTY)]
    EmptyVersion,
    /// The install package trims to empty.
    #[error("{}", NON_EMPTY)]
    EmptyPackage,
    /// A present install source trims to empty.
    #[error("{}", NON_EMPTY)]
    EmptySource,
    /// The tool binary is spelled with a path.
    #[error("tool binary '{binary}' must be a bare filename resolved from PATH")]
    NotBare { binary: &'a str },
    /// An artifact phase has no steps.
    #[error("phase '{phase}' is unfilled")]
    Unfilled { phase: &'a str },
    /// Every step of an artifact phase is optional.
    #[error("phase '{phase}' has no required step")]
    NoRequiredStep { phase: &'a str },
    /// A step's name trims to empty.
    #[error("phase '{phase}' has an empty step or artifact list")]
    UnnamedStep { phase: &'a str },
    /// A step assigns no artifact.
    #[error("phase '{phase}' has an empty step or artifact list")]
    NoArtifacts { phase: &'a str },
    /// An artifact is assigned by a second step or twice by one.
    #[error("artifact '{artifact}' is assigned more than once")]
    AssignedTwice { artifact: &'a str },
    /// An edge's `before` names no assigned artifact.
    #[error("order names unassigned artifact '{artifact}'")]
    UnassignedBefore { artifact: &'a str },
    /// An edge's `after` names no assigned artifact.
    #[error("order names unassigned artifact '{artifact}'")]
    UnassignedAfter { artifact: &'a str },
    /// An edge's dependency is mapped at or after its dependent.
    #[error("artifact '{after}' is mapped before its dependency '{before}'")]
    Reversed { after: &'a str, before: &'a str },
    /// A loop phase's taxonomy trims to empty.
    #[error("phase '{phase}' has no taxonomy")]
    NoTaxonomy { phase: &'a str },
    /// An instruction path is absolute or climbs out of its dialect's folder.
    #[error("instruction path '{instruction}' must be relative and remain beside its dialect")]
    Escapes { instruction: &'a str },
    /// A command token holds a brace outside `{change}`.
    #[error("argv token '{token}' uses an unknown placeholder")]
    UnknownPlaceholder { token: &'a str },
}

/// The refusal of the dialect at `path`, as the operator reads it.
pub(super) fn refused(path: &str, refusal: DialectRefusal<'_>) -> DialectError {
    DialectError::Invalid {
        path: path.to_string(),
        problem: refusal.to_string(),
    }
}

#[cfg(test)]
mod tests;
