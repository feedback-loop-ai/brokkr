//! Why the pnpm reader's two leaf grammars refused: the `resolution:` flow
//! map and the `packages:` key (#353). Each check has its own variant, so a
//! test tells which one fired, and each `Display` is the text the reader
//! has always written, so the `CompositeError::PnpmLock` built from it, and
//! every journaled byte, does not move.

use thiserror::Error;

/// Why a `resolution:` flow map's body cannot be read. Four checks share
/// the words "a malformed resolution flow map"; the variant names which
/// one fired.
#[derive(Debug, PartialEq, Eq, Error)]
pub(super) enum ResolutionRefusal<'a> {
    /// The body holds a brace: a nested collection.
    #[error("a malformed resolution flow map")]
    Nested,
    /// A field with no `:` at all.
    #[error("a malformed resolution flow map")]
    Unkeyed { field: &'a str },
    /// A key outside `PNPM_RESOLUTION_KEYS`.
    #[error("a malformed resolution flow map")]
    UnknownKey { key: &'a str },
    /// A value that is not a scalar: `FlowFault::Malformed`.
    #[error("a malformed resolution flow map")]
    Value { key: &'a str },
    /// A `:` not followed by the space a flow mapping requires.
    #[error("the resolution field '{key}' lacks ': ' separation: '{field}'")]
    Unseparated { key: &'a str, field: &'a str },
    /// A plain scalar carrying the collection's own punctuation.
    #[error("the resolution field '{key}' carries unsupported flow syntax: '{value}'")]
    Punctuation { key: &'a str, value: &'a str },
    /// An `integrity` plain scalar YAML reads as another type.
    #[error(
        "the resolution field '{key}' is the plain scalar '{text}', \
         which is {kind} and not a string"
    )]
    Typed {
        key: &'a str,
        text: &'a str,
        kind: &'static str,
    },
}

/// Why a `packages:` key does not split into a package name and version.
#[derive(Debug, PartialEq, Eq, Error)]
pub(super) enum PackageKeyRefusal<'a> {
    /// The whole key fails `scalar_reason`.
    #[error("'{key}': the key {reason}")]
    Scalar { key: &'a str, reason: &'static str },
    /// No `@` after the key's first character.
    #[error("'{key}': no '@' after the first character")]
    NoAt { key: &'a str },
    /// Nothing after the separating `@`.
    #[error("'{key}': empty version")]
    NoVersion { key: &'a str },
    /// The name half fails the package-name grammar.
    #[error("'{key}': '{name}' is not a package name")]
    NotAName { key: &'a str, name: &'a str },
}

#[cfg(test)]
mod tests;
