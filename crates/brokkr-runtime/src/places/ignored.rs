//! Git's own answer to whether a path under a bundle is ignored (#908),
//! split from `places.rs` so that file stays within its ceiling (decision
//! 0071 ruling 4).

use std::path::Path;

use super::{SkipError, GIT};

/// Whether `dir` has a `.git` entry at itself or an ancestor: the one
/// question that tells a bundle outside any git work tree, which asks git
/// nothing, from one whose answer must be read (#908).
fn stands_in_git(dir: &Path) -> bool {
    dir.ancestors()
        .any(|at| at.join(GIT).symlink_metadata().is_ok())
}

/// Git's own answer to whether `rel` under `dir` is ignored (#908):
/// `git check-ignore` for the repository the bundle stands in, so
/// `.gitignore`, `.git/info/exclude` and a global exclude rule alike are
/// git's rules, never a copy of them. `rel` is the entry's full relative
/// path, so a tree ignored at any depth is judged, not only the first
/// component. It is passed to git as `./<rel>`, so git judges the literal
/// name the walk named and never a pathspec (#908).
///
/// A bundle outside a git work tree answers "not ignored": it skips only
/// [`GIT`]. A `.git` stands above the bundle but git cannot answer for it:
/// that refusal is by name, with git's own stderr, never read as "nothing
/// is ignored" or "everything is".
pub(super) fn git_ignored(dir: &Path, rel: &str) -> Result<bool, SkipError> {
    // A directory that is no longer there has no repository to ask: the
    // read that follows names it missing, as it always did, rather than
    // asking git about a bundle that does not stand.
    if !dir.exists() {
        return Ok(false);
    }
    // A directory with no `.git` at it or above it stands in no work tree,
    // so there is no question to ask and it skips only `.git`, as it did
    // before #908. A path that is not a directory is still asked, so git's
    // own refusal names it rather than being read as "not ignored".
    if dir.is_dir() && !stands_in_git(dir) {
        return Ok(false);
    }
    // Run git in the bundle, as the house's other git calls do. A bundle
    // whose directory cannot be entered is a spawn failure, refused by
    // name rather than read as "nothing is ignored".
    let run = |args: &[&str]| -> Result<std::process::Output, SkipError> {
        std::process::Command::new("git")
            .current_dir(dir)
            .args(args)
            .output()
            .map_err(|error| SkipError::Git {
                bundle: dir.to_path_buf(),
                detail: error.to_string(),
            })
    };
    // Git's own test for the question. A repository git cannot read is a
    // refusal by name carrying git's stderr: only a successful `false`
    // answer says the bundle is inside no work tree (a bare repository or
    // git's own metadata), which pins as it always did.
    let inside = run(&["rev-parse", "--is-inside-work-tree"])?;
    if !inside.status.success() {
        return Err(SkipError::Git {
            bundle: dir.to_path_buf(),
            detail: git_refusal(&inside, "rev-parse"),
        });
    }
    if String::from_utf8_lossy(&inside.stdout).trim() != "true" {
        return Ok(false);
    }
    // Git reads a bare relative operand as a pathspec, and `--` ends
    // options without making it literal: a `:`-leading name is pathspec
    // magic, so git would judge a different path than the walk named.
    // Prefixing `./` judges the literal name relative to the bundle git
    // already runs in (`--literal-pathspecs` and `--stdin` are no help,
    // and check-ignore expands no glob) (#908).
    //
    // A bundle whose own root git ignores stands outside the repository's
    // tracked content: it is judged as a bundle outside any work tree, so
    // it skips only `.git` and nothing inside it is skipped as ignored
    // (operator, 2026-11-01, #908). Asked first, as `.`, the bundle itself.
    if check_ignore(dir, &run, ".")? {
        return Ok(false);
    }
    check_ignore(dir, &run, &format!("./{rel}"))
}

/// `git check-ignore --quiet -- <operand>` in `dir`: 0 is ignored, 1 is
/// not, and any other answer is a refusal by name with git's own words.
fn check_ignore(
    dir: &Path,
    run: &dyn Fn(&[&str]) -> Result<std::process::Output, SkipError>,
    operand: &str,
) -> Result<bool, SkipError> {
    let checked = run(&["check-ignore", "--quiet", "--", operand])?;
    match checked.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(SkipError::Git {
            bundle: dir.to_path_buf(),
            detail: git_refusal(&checked, "check-ignore"),
        }),
    }
}

/// Git's own words for an answer it could not give (#908): its status and
/// stderr, so the refusal names what git said rather than guessing.
pub(super) fn git_refusal(output: &std::process::Output, verb: &str) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stderr = stderr.trim();
    if stderr.is_empty() {
        format!("git {verb} answered {}", output.status)
    } else {
        format!("git {verb} answered {}: {stderr}", output.status)
    }
}
