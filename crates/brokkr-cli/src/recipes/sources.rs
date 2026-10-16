//! The planned recipes `recipes add` refuses to copy (#696): a repository's
//! folder bundle, its default bundle, is no recipe. Its directory holds the
//! folder's journal under `state/`, its agents, its adapters and its own
//! shelf beside its files, and an install copies a source whole, so a
//! private journal and a nested shelf would land on the destination's
//! shelf. Refused, every recipe the plan holds takes its bases from beside
//! it, as the compiler composes a layer that is no folder bundle.

use std::path::{Path, PathBuf};

use brokkr_runtime::places::{self, FOLDER, RECIPES};

/// An `add` refused because `source`, a recipe it planned, the leaf or a
/// base, is a folder repository's `.brokkr` folder, its default bundle
/// ([`places::folder_bundle`]). A caller matches the type, never the text
/// (decision 0071 ruling 8).
#[derive(Debug, PartialEq, Eq)]
pub(super) struct FolderBundle {
    pub(super) source: PathBuf,
}

impl std::fmt::Display for FolderBundle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let source = self.source.display();
        write!(
            formatter,
            "{source} is a repository's {FOLDER} folder, its default bundle, not a recipe: name a \
             recipe on its shelf ({source}/{RECIPES}/<name>) or in a library as the source \
             (#696); nothing was written"
        )
    }
}

impl std::error::Error for FolderBundle {}

/// Refuse when any of `sources`, the recipes an install planned, is a
/// folder bundle, each judged where it resolves, however it was reached:
/// spelled, through a link, or as a clone's root or `--path`.
pub(super) fn refuse_folder_bundles(sources: &[&Path]) -> anyhow::Result<()> {
    for bundle in sources {
        let source = bundle.canonicalize()?;
        if places::folder_bundle(&source)?.is_some() {
            return Err(FolderBundle { source }.into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod sources_tests;
