//! Why an unboxed exec site's pinned script was refused (decision 0046
//! ruling 4; decision 0048; decision 0049 ruling 3).
//!
//! `pinned_script` and its lookup half `pinned_key` refuse in this one
//! cause, so a test can name the check that fired. Its `Display` is the
//! operator's text byte for byte; `inline_hands_boundary` folds it into
//! the compile refusal, the one place it becomes text (#353).

use std::path::Path;

use crate::places::SkipError;

/// Why a pinned script was refused, each field borrowed from the argv,
/// its script token or the declaring layer's directory.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub(super) enum PinnedScriptRefusal<'a> {
    /// The argv has no `--` ending `{brokkr} driver exec`.
    #[error("the dispatch carries no `--` after `exec`")]
    NoSeparator,
    /// An option token stands before the script.
    #[error(
        "'{token}' is an option token before the script; interpreters take no options here \
         (`bash -c` would run text, not pinned bytes)"
    )]
    OptionToken { token: &'a str },
    /// A path token that is not `./`-relative.
    #[error(
        "'{token}' is spelled as a path that is not `./`-relative to the bundle; only a `./` \
         script the manifest walk pins is admitted unboxed"
    )]
    Spelled { token: &'a str },
    /// The argv after `--` holds bare interpreter names only.
    #[error(
        "the command names no `./`-relative script after its interpreters ({})",
        .after.join(" ")
    )]
    NoScript { after: &'a [String] },
    /// A component holds one of decision 0048's startup metacharacters.
    #[error(
        "'{token}' has component {component:?} containing refused startup character \
         {character:?}; rename the component to meet the pinned-script admission rule \
         (decision 0048; decision 0046 ruling 4)"
    )]
    Startup {
        token: &'a str,
        component: &'a str,
        character: char,
    },
    /// A component is empty, `.`, `..` or a drive.
    #[error(
        "'{token}' is not a plain `./`-relative path: every component is a plain name, never \
         `..`, `.`, empty, a `\\` or a drive"
    )]
    NotPlain { token: &'a str },
    /// Git could not say whether the walk pins the script.
    #[error(transparent)]
    Git(SkipError),
    /// The manifest walk skips the script's path.
    #[error("'{token}' names a path the manifest walk does not pin ({relative} is {holds})")]
    Unpinned {
        token: &'a str,
        relative: &'a str,
        holds: &'static str,
    },
    /// No regular file stands at the script's path.
    #[error(
        "'{token}' names no regular file under the declaring layer {}",
        .dir.display()
    )]
    NotAFile { token: &'a str, dir: &'a Path },
}

#[cfg(test)]
mod tests;
