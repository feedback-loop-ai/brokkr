//! Why a secret's name, its value or a `{{secret:NAME}}` reference was
//! refused: one variant per check, each reading as the text the public
//! surfaces have always printed. The three public functions in the parent
//! turn a refusal into its `String` and nowhere else does. A refusal names
//! a binding name or a template's reference text, never a secret's value
//! (decision 0012).

use super::MIN_VALUE_BYTES;

/// Why a name cannot be bound, in the order the checks run.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub(super) enum NameRefusal<'a> {
    #[error("secret name '{name}' does not match [A-Z][A-Z0-9_]*")]
    Grammar { name: &'a str },
    #[error(
        "secret name '{name}' is denylisted (PATH, IFS, LD_PRELOAD, \
         LD_LIBRARY_PATH, and the BROKKR_ prefix are never bindable)"
    )]
    Denylisted { name: &'a str },
}

/// Why a value cannot be stored, in the order the checks run.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub(super) enum ValueRefusal {
    #[error("secret value is empty")]
    Empty,
    #[error("secret value must be a single line (the store is line-oriented)")]
    MultiLine,
    #[error("secret value must not contain NUL")]
    Nul,
    #[error(
        "secret value is shorter than {} bytes; masking a \
         tiny value shreds the journal's evidence trail",
        MIN_VALUE_BYTES
    )]
    TooShort,
}

/// Why a template's first malformed reference was refused. Whitespace
/// prints two braces, `near '{{ secret:'`, like its neighbours: main built
/// it from a plain literal, not a format string, so the escapes it wrote
/// printed as four, and the operator ruled on 2026-11-21 that two were
/// meant.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub(super) enum ReferenceRefusal<'a> {
    #[error(
        "malformed secret reference near '{{{{ secret:': whitespace \
         is not allowed inside a reference"
    )]
    Whitespace,
    #[error(
        "malformed secret reference '{quoted}': expected {{{{secret:NAME}}}} \
         with NAME matching [A-Z][A-Z0-9_]*"
    )]
    Name { quoted: &'a str },
    #[error(
        "malformed secret reference '{{{{secret:{name}': missing the \
         closing '}}}}' (or the name continues with characters outside \
         [A-Z0-9_])"
    )]
    Unclosed { name: &'a str },
}

#[cfg(test)]
mod tests;
