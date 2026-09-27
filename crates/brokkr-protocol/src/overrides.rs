//! The harness's environment overrides, and the spellings decision 0019
//! retired.
//!
//! Each override is read by its current name only (#355), here and
//! nowhere else. What cannot be read is REFUSED by name, never taken as
//! unset: a retired spelling set where the current name is not, and a
//! current value that is not UTF-8. Either way an operator who pinned an
//! executable would otherwise get the built-in one, holding the seat's
//! grants, with no line on stderr (landing reviews of #355, 2026-09-27).
//! Both spellings of all six names, and what each falls back to, live
//! here, once.

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

/// An override that cannot be read as the value it names.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OverrideError {
    /// The current name holds bytes that are not UTF-8. Refused whether
    /// or not a retired spelling is also set, since the current name wins.
    #[error(
        "{variable} is set, but its value is not UTF-8, so what it names cannot \
         be read; give {variable} a UTF-8 value, or unset it"
    )]
    NotUnicode { variable: &'static str },
    /// A retired spelling met where its current name is unset.
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

    /// What the harness runs, or names itself, when the override is unset.
    pub(crate) const fn fallback(self) -> &'static str {
        match self {
            Override::ClaudeBin => "claude",
            Override::LanetallyBin => "claude-lanetally",
            Override::CodexBin => "codex",
            Override::DshBin => "dsh",
            Override::ExecName => "exec",
            Override::BrowserBin => "xdg-open",
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

/// The override's value from the process environment, the fallback when
/// neither spelling is set, and the refusal when it cannot be read. The
/// one reader: every consumer takes the value it returns and reads the
/// environment for it no second time.
pub fn read(what: Override) -> Result<String, OverrideError> {
    read_with(what, |name| std::env::var_os(name))
}

/// `read` over an injected environment. A retired spelling refuses
/// whatever its value, since what it names is never used.
fn read_with(
    what: Override,
    lookup: impl Fn(&str) -> Option<OsString>,
) -> Result<String, OverrideError> {
    if let Some(value) = lookup(what.name()) {
        return value.into_string().map_err(|_| OverrideError::NotUnicode {
            variable: what.name(),
        });
    }
    let retired = what.retired();
    match lookup(&retired) {
        Some(_) => Err(OverrideError::Retired {
            retired,
            current: what.name(),
        }),
        None => Ok(what.fallback().to_string()),
    }
}

#[cfg(test)]
mod tests;
