//! Why a realms map that parses is not a usable one (#353): each refusal
//! is a variant, and its `Display` is the text the operator has always
//! read, byte for byte.

use thiserror::Error;

use super::grants::GrantError;
use super::{
    BoundaryError, SCHEMA_V1, SCHEMA_V2, SCHEMA_V3, SCHEMA_V4, SCHEMA_V5, SCHEMA_V6, SCHEMA_V7,
    SCHEMA_V8,
};

/// A per-realm word a version introduced, held to the version that
/// introduced it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Word {
    Journal,
    House,
    Dialect,
    Boundary,
    Publishes,
    Consumes,
    Capabilities,
}

impl Word {
    /// The field as the map spells it.
    pub(crate) fn field(self) -> &'static str {
        match self {
            Word::Journal => "journal",
            Word::House => "house",
            Word::Dialect => "dialect",
            Word::Boundary => "boundary",
            Word::Publishes => "publishes",
            Word::Consumes => "consumes",
            Word::Capabilities => "capabilities",
        }
    }

    /// The version that introduced the word.
    pub(crate) fn introduced(self) -> &'static str {
        match self {
            Word::Journal => SCHEMA_V2,
            Word::House | Word::Dialect => SCHEMA_V3,
            Word::Boundary => SCHEMA_V4,
            Word::Publishes | Word::Consumes => SCHEMA_V5,
            Word::Capabilities => SCHEMA_V6,
        }
    }

    /// What a realm that writes the word is said to name.
    fn named(self) -> String {
        match self {
            Word::Journal => "its own journal".to_string(),
            Word::Publishes | Word::Consumes => format!("what it {}", self.field()),
            Word::House | Word::Dialect | Word::Boundary | Word::Capabilities => {
                format!("its {}", self.field())
            }
        }
    }
}

/// A realm's name as a refusal prints it: `?` where it wrote no string.
fn unnamed(realm: &Option<String>) -> &str {
    realm.as_deref().unwrap_or("?")
}

/// Why a realms map is refused once it parses: its version, its realms,
/// their crossings and grants, and the world's provisional offices.
#[derive(Debug, PartialEq, Eq, Error)]
pub enum Unusable {
    /// A boundary word outside the five; `realm` is `None` when the realm
    /// that wrote it has no string name.
    #[error("realm '{}' declares boundary {error}", unnamed(.realm))]
    Boundary {
        realm: Option<String>,
        error: BoundaryError,
    },
    #[error(
        "it calls itself '{0}'; this build reads {SCHEMA_V1}, {SCHEMA_V2}, {SCHEMA_V3}, \
         {SCHEMA_V4}, {SCHEMA_V5}, {SCHEMA_V6}, {SCHEMA_V7} and {SCHEMA_V8}"
    )]
    UnknownSchema(String),
    #[error(
        "realm '{realm}' names {}, which is {} vocabulary in a map calling itself {schema}",
        .word.named(),
        .word.introduced()
    )]
    Unversioned {
        realm: String,
        word: Word,
        schema: String,
    },
    #[error("it names no realms")]
    NoRealms,
    #[error("its journal is empty")]
    EmptyJournal,
    #[error(
        "realm {index} is named '{name}'; a realm name is lowercase letters, digits, '.', '_' \
         and '-', starting with a letter or digit"
    )]
    Name { index: usize, name: String },
    #[error("realm '{0}' has no path")]
    NoPath(String),
    #[error("realm '{0}' has no default branch")]
    NoBranch(String),
    #[error("realm '{0}' is named twice")]
    NamedTwice(String),
    #[error("realm '{realm}' has an empty {}", .word.field())]
    Empty { realm: String, word: Word },
    #[error("realm '{realm}' has a non-repository-relative {}", .word.field())]
    Outside { realm: String, word: Word },
    #[error(
        "realm '{realm}' writes {} as null; a crossing list is an array, and a realm that \
         draws no crossing leaves the word out",
        .word.field()
    )]
    NullList { realm: String, word: Word },
    #[error(
        "realm '{realm}' publishes a crossing named '{crossing}'; a crossing name is lowercase \
         letters, digits, '.', '_' and '-', starting with a letter or digit"
    )]
    CrossingName { realm: String, crossing: String },
    #[error("realm '{realm}' publishes crossing '{crossing}' with no path")]
    NoCrossingPath { realm: String, crossing: String },
    #[error("realm '{realm}' publishes crossing '{crossing}' from a non-repository-relative path")]
    CrossingOutside { realm: String, crossing: String },
    #[error("realm '{realm}' publishes a crossing named '{crossing}' twice")]
    PublishedTwice { realm: String, crossing: String },
    #[error(
        "realm '{realm}' pins crossing '{crossing}' at '{pin}', which is not a sha256: a pin \
         is 64 lowercase hex characters over the published file's raw bytes"
    )]
    Pin {
        realm: String,
        crossing: String,
        pin: String,
    },
    #[error(
        "realm '{realm}' consumes crossing '{crossing}' from itself; a crossing is between \
         realms, and a realm reads its own file as a file"
    )]
    ConsumesItself { realm: String, crossing: String },
    #[error("realm '{realm}' consumes a crossing named '{crossing}' twice")]
    ConsumedTwice { realm: String, crossing: String },
    #[error(
        "realm '{realm}' consumes crossing '{crossing}' from realm '{publisher}', which this \
         world does not hold"
    )]
    NoPublisher {
        realm: String,
        crossing: String,
        publisher: String,
    },
    #[error(
        "realm '{realm}' consumes crossing '{crossing}', which realm '{publisher}' does not \
         publish"
    )]
    Unpublished {
        realm: String,
        crossing: String,
        publisher: String,
    },
    #[error(transparent)]
    Grant(#[from] GrantError),
    #[error(
        "it names provisional offices, which is {SCHEMA_V7} vocabulary in a map calling \
         itself {0}"
    )]
    ProvisionalUnversioned(String),
    #[error(
        "it writes provisional_offices as null; the list is an array, and a map that lists no \
         office leaves the word out"
    )]
    ProvisionalNull,
    #[error("provisional office {0} is empty")]
    EmptyOffice(usize),
    #[error("provisional office '{0}' is listed twice")]
    OfficeTwice(String),
}
