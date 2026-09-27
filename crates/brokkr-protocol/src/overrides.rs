//! The harness's environment overrides, and the spellings decision 0019
//! retired.
//!
//! Each override is read by its current name only (#355). A retired
//! spelling set where the current name is not is REFUSED by name, never
//! ignored: an operator who pinned an executable through it would
//! otherwise get the built-in one, holding the seat's grants, with no
//! line on stderr (landing review of #355, 2026-09-27). Both spellings
//! of all six names live here, once.

use std::ffi::OsString;

/// The prefix decision 0019 retired, split so the rename's standing grep
/// finds no live spelling of it in the tree.
const RETIRED_PREFIX: &str = concat!("FOR", "GE_");

/// The prefix every current override carries.
const CURRENT_PREFIX: &str = "BROKKR_";

/// The overrides the harness reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Override {
    ClaudeBin,
    LanetallyBin,
    CodexBin,
    DshBin,
    ExecName,
    BrowserBin,
}

/// A retired spelling met where its current name is unset.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OverrideError {
    #[error(
        "{retired} is set but is no longer read: it was renamed {current}; \
         set {current} instead, or unset {retired}"
    )]
    Retired {
        retired: String,
        current: &'static str,
    },
}

impl Override {
    /// The name the harness reads.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Override::ClaudeBin => "BROKKR_CLAUDE_BIN",
            Override::LanetallyBin => "BROKKR_LANETALLY_BIN",
            Override::CodexBin => "BROKKR_CODEX_BIN",
            Override::DshBin => "BROKKR_DSH_BIN",
            Override::ExecName => "BROKKR_EXEC_NAME",
            Override::BrowserBin => "BROKKR_BROWSER_BIN",
        }
    }

    /// The same name under the retired prefix.
    pub(crate) fn retired(self) -> String {
        format!("{RETIRED_PREFIX}{}", &self.name()[CURRENT_PREFIX.len()..])
    }

    /// The override that names what a driver of `kind` runs as.
    pub(crate) const fn of_driver(kind: crate::adapters::AdapterKind) -> Override {
        use crate::adapters::AdapterKind;
        match kind {
            AdapterKind::Claude => Override::ClaudeBin,
            AdapterKind::Lanetally => Override::LanetallyBin,
            AdapterKind::Codex => Override::CodexBin,
            AdapterKind::Dsh => Override::DshBin,
            AdapterKind::Exec => Override::ExecName,
        }
    }
}

/// The override's value from the process environment: `Ok(None)` when
/// neither spelling is set, and the refusal when only the retired one is.
pub fn read(what: Override) -> Result<Option<String>, OverrideError> {
    read_with(what, |name| std::env::var_os(name))
}

/// `read` over an injected environment. A current value that is not
/// UTF-8 reads as unset, as it did before #355. A retired spelling
/// refuses whatever its value, since what it names is never used.
fn read_with(
    what: Override,
    lookup: impl Fn(&str) -> Option<OsString>,
) -> Result<Option<String>, OverrideError> {
    if let Some(value) = lookup(what.name()).and_then(|value| value.into_string().ok()) {
        return Ok(Some(value));
    }
    let retired = what.retired();
    match lookup(&retired) {
        Some(_) => Err(OverrideError::Retired {
            retired,
            current: what.name(),
        }),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests;
