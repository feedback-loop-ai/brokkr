//! Why a route overlay is refused (#353): each refusal is a variant, and
//! its `Display` is the text the seat's launch refusal has always carried,
//! byte for byte. A refusal names a field, a part or a line, never a value.

use std::fmt;

use thiserror::Error;

/// A field the closed route grammar names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::adapters) enum Field {
    Id,
    Config,
    Providers,
    DisplayName,
    Api,
    BaseUrl,
    Compat,
    Models,
    ApiKeyEnv,
    ReasoningEfforts,
}

impl Field {
    /// The field as a route spells it.
    pub(super) fn spelling(self) -> &'static str {
        match self {
            Field::Id => "id",
            Field::Config => "config",
            Field::Providers => "providers",
            Field::DisplayName => "displayName",
            Field::Api => "api",
            Field::BaseUrl => "baseURL",
            Field::Compat => "compat",
            Field::Models => "models",
            Field::ApiKeyEnv => "apiKeyEnv",
            Field::ReasoningEfforts => "reasoningEfforts",
        }
    }
}

impl fmt::Display for Field {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.spelling())
    }
}

/// A part of the route a semantic check reads: one of its three mappings,
/// or the value of a named field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::adapters) enum Part {
    RouteEntry,
    ProviderMapping,
    ModelItem,
    HandsRow,
    Named(Field),
}

impl fmt::Display for Part {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Part::RouteEntry => "the route entry",
            Part::ProviderMapping => "the provider mapping",
            Part::ModelItem => "the model item",
            Part::HandsRow => "a hands row",
            Part::Named(field) => field.spelling(),
        })
    }
}

/// The refusal a seat's launch carries for its route overlay.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("refusing to invoke the dsh driver: {0}")]
pub(in crate::adapters) struct Refused(pub(in crate::adapters) Overlay);

/// By hand, not `#[from]`: `#[from]` makes the reason the error's source,
/// and a `{:#}` chain would print its text a second time.
impl From<Overlay> for Refused {
    fn from(reason: Overlay) -> Self {
        Refused(reason)
    }
}

/// The launch refusal is still text in `adapters.rs` (decision 0065 slice
/// two holds that file), so the conversion is the boundary's.
impl From<Refused> for String {
    fn from(refused: Refused) -> Self {
        refused.to_string()
    }
}

/// Why the binding, the file or its bytes are refused.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(in crate::adapters) enum Overlay {
    #[error("a `--patch` with no bound route overlay")]
    PatchUnbound,
    #[error("route_overlay is not an object")]
    BindingNotAnObject,
    #[error("route_overlay carries no value")]
    NoValue,
    #[error("route_overlay carries no digest")]
    NoDigest,
    #[error("the route overlay binding disagrees with the `--patch` value")]
    Disagrees,
    #[error("a route overlay needs a pinned `--model` with a provider segment")]
    NoProviderPin,
    #[error("route_overlay value is empty")]
    EmptyValue,
    #[error("route_overlay value is an absolute path")]
    Absolute,
    #[error("route_overlay value contains a `..` component")]
    ParentComponent,
    #[error("route_overlay working directory is unreadable")]
    WorkdirUnreadable,
    #[error("route_overlay file is unreadable")]
    FileUnreadable,
    #[error("route_overlay value resolves outside the working directory")]
    Outside,
    #[error("route_overlay value is not a regular file")]
    NotRegular,
    #[error("route_overlay exceeds the reader's byte bound")]
    FileOverBound,
    #[error("route_overlay bytes do not hash to the bound manifest digest")]
    DigestMismatch,
    #[error("route overlay is not UTF-8")]
    NotUtf8,
    #[error("route overlay must hold exactly one top-level entry")]
    EntryCount,
    #[error("route overlay must be a sequence of one entry")]
    NotASequence,
    #[error("the route entry id is not the route row")]
    NotTheRouteRow,
    #[error("providers must define exactly one provider")]
    ProviderCount,
    #[error("the pinned model is not `<id>` or `<provider>/<id>`")]
    PinGrammar,
    #[error("the route names a provider the seat did not pin")]
    UnpinnedProvider,
    #[error("the provider mapping repeats a field")]
    ProviderFieldRepeated,
    #[error("apiKeyEnv is not an environment-variable name")]
    ApiKeyEnv,
    #[error("baseURL leaves the closed endpoint grammar")]
    Endpoint,
    #[error("models must hold exactly one item")]
    ModelCount,
    #[error("models is not a sequence")]
    ModelsNotASequence,
    #[error("the route names a model the seat did not pin")]
    UnpinnedModel,
    #[error("route overlay exceeds the reader's byte bound")]
    TextOverBound,
    #[error("route overlay line {0} carries a tab or control character")]
    Control(usize),
    #[error("route overlay line {0} is a document marker")]
    DocumentMarker(usize),
    #[error("route overlay line {0} has an odd indentation")]
    OddIndentation(usize),
    #[error("route overlay has too many lines")]
    TooManyLines,
    #[error("route overlay is empty")]
    Empty,
    #[error("route overlay line {0} has an unexpected indentation")]
    UnexpectedIndentation(usize),
    #[error("route overlay indentation is inconsistent")]
    Inconsistent,
    #[error("route overlay block has no lines")]
    EmptyBlock,
    #[error("route overlay line {0} is a bare sequence item")]
    BareItem(usize),
    #[error("route overlay sequence is empty")]
    EmptySequence,
    #[error("route overlay line {0} jumps more than one depth")]
    DepthJump(usize),
    #[error("route overlay line {line} has an empty `{key}:` block")]
    EmptyKeyBlock { line: usize, key: String },
    #[error("route overlay line {0} is not a recognized block line")]
    Unrecognized(usize),
    #[error("route overlay line {0} carries a key that is not a plain identifier")]
    KeyNotIdentifier(usize),
    #[error("route overlay line {0} carries an empty value")]
    EmptyScalar(usize),
    #[error("route overlay line {0} carries a value beginning with a reserved character")]
    ReservedStart(usize),
    #[error("route overlay line {0} carries a value with a comment or an extra mapping")]
    CommentOrMapping(usize),
    #[error("{0} is not a mapping")]
    NotAMapping(Part),
    #[error("{part} is missing `{field}`")]
    Missing { part: Part, field: Field },
    #[error("{part} repeats `{field}`")]
    Repeats { part: Part, field: Field },
    #[error("{part} nests a non-scalar `{field}`")]
    NonScalar { part: Part, field: Field },
    #[error("{part} nests a non-mapping `{field}`")]
    NonMapping { part: Part, field: Field },
    #[error("{0} carries a field outside the closed set")]
    OutsideClosedSet(Part),
    #[error("the hands rows carry row '{0}' in a shape the measured hands set does not name")]
    HandsRow(String),
    #[error("the hands rows name row '{0}' twice")]
    HandsRowRepeated(String),
    #[error("the hands rows carry no row '{0}', which the measured hands set names")]
    HandsRowMissing(&'static str),
}
