//! Decision 0009's argv convention, read once (#349, ask 8): a built-in
//! driver is dispatched as `<engine> driver <kind> -- …`, and the engine
//! token itself is never matched on — a bundle may spell it `{brokkr}` or
//! the absolute path of the binary it means, and both are the same
//! dispatch. [`Dispatch`] is that reading, and [`Executable`] is the engine
//! token's value: resolved once by the entry point that compiles or runs,
//! and handed to the compile's `{brokkr}` expansion and to the engine's
//! every spawn that names it.

use std::path::{Path, PathBuf};

/// The literal that marks a built-in dispatch, behind the engine token.
const DRIVER: &str = "driver";

/// The escape a dispatch may write directly behind its verb.
const ESCAPE: &str = "--";

/// One argv as decision 0009's convention reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Dispatch<'a> {
    /// `<engine> driver <kind> …`: the driver kind, and what the command
    /// hands it behind the verb, less one escape `--` directly behind it.
    Driver { kind: &'a str, tail: &'a [String] },
    /// Any other argv — a raw process, an empty one, or one too short to
    /// name a kind — dispatches no built-in driver.
    Raw,
}

impl<'a> Dispatch<'a> {
    /// `argv` read as a dispatch.
    pub(crate) fn of(argv: &'a [String]) -> Dispatch<'a> {
        match argv {
            [_, marker, kind, rest @ ..] if marker == DRIVER => Dispatch::Driver {
                kind: kind.as_str(),
                tail: match rest {
                    [escape, tail @ ..] if escape == ESCAPE => tail,
                    _ => rest,
                },
            },
            _ => Dispatch::Raw,
        }
    }

    /// Whether this is the engine's own `exec` driver.
    pub(crate) fn is_exec(self) -> bool {
        matches!(self, Dispatch::Driver { kind: "exec", .. })
    }
}

/// The engine's own executable, as `std::env::current_exe` answered it
/// once for the entry point that holds this value: `None` when the
/// platform could not say. Machine-local, so never digested; each reader
/// keeps the spelling it has always given an unreadable one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Executable(pub(crate) Option<PathBuf>);

impl Executable {
    /// This process's executable, read once by an entry point that compiles
    /// or runs and passed from there to everything that names it.
    pub fn current() -> Executable {
        Executable(std::env::current_exe().ok())
    }

    /// The path, when it was readable.
    pub(crate) fn path(&self) -> Option<&Path> {
        self.0.as_deref()
    }

    /// The path the engine composes a spawn with: the empty path when it was
    /// unreadable, as the engine has always spelled it.
    pub(crate) fn or_empty(&self) -> &Path {
        self.path().unwrap_or(Path::new(""))
    }

    /// [`Executable::or_empty`] as the argv token a spawn carries.
    pub(crate) fn lossy(&self) -> String {
        self.or_empty().to_string_lossy().into_owned()
    }
}

#[cfg(test)]
pub(crate) mod tests;
