//! Why a `hands.binds` entry was refused (decision 0043; decision 0046).
//!
//! `Bind::checked` refuses in this one cause, one variant per check, so a
//! test can name the check that fired and tell a bind's refusal from the
//! spec's. Its `Display` is the operator's text byte for byte;
//! `HandsSpec::parse` is the one place it becomes text (#353).

/// Why a bind entry was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BindRefusal {
    /// The entry is not a JSON object.
    #[error("hands.binds entry must be an object")]
    NotObject,
    /// The entry names a key outside `path`, `mode` and `mask`.
    #[error("hands.binds entry has unknown key '{key}'; known: path, mode, mask")]
    UnknownKey { key: String },
    /// The entry's `path` is absent, not a string, or empty.
    #[error("hands.binds entry needs a non-empty 'path'")]
    NoPath,
    /// The entry's `mode` is none of `ro`, `rw` and `overlay`.
    #[error("hands.binds entry '{path}' needs mode \"ro\", \"rw\" or \"overlay\"")]
    Mode { path: String },
    /// The entry's `mask` is not an array.
    #[error("hands.binds entry '{path}' mask must be an array")]
    MaskNotArray { path: String },
    /// A mask name is not a string, is empty, or holds a slash.
    #[error(
        "hands.binds entry '{path}' mask names one file each, \
         without a slash"
    )]
    MaskName { path: String },
}

impl BindRefusal {
    /// Stable variant name with no field name or value taken from the
    /// rejected bind entry (#1277).
    pub fn variant_name(&self) -> &'static str {
        match self {
            Self::NotObject => "Bind::NotObject",
            Self::UnknownKey { .. } => "Bind::UnknownKey",
            Self::NoPath => "Bind::NoPath",
            Self::Mode { .. } => "Bind::Mode",
            Self::MaskNotArray { .. } => "Bind::MaskNotArray",
            Self::MaskName { .. } => "Bind::MaskName",
        }
    }
}

#[cfg(test)]
mod tests;
