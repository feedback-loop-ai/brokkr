//! Seed host policy only on a fresh setup (decision 0125, ask E).

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use brokkr_runtime::admission::Place;
use brokkr_runtime::Bundle;
use serde::Serialize;
use tempfile::NamedTempFile;

use super::{host_upgrade, parent, read, read_error, stage, UpgradeError};
use super::{CURRENT_HOST_SCHEMA, UPGRADE_COMMAND};

const PLACEHOLDER_CEILING: u32 = 1;
const SCRATCH_FLOOR_BYTES: u64 = 30_000_000_000;

#[derive(Serialize)]
struct Draft<'a> {
    schema: &'static str,
    providers: BTreeMap<&'a str, Ceiling>,
    floors: [Floor; 1],
}

#[derive(Serialize)]
struct Ceiling {
    ceiling: u32,
}

#[derive(Serialize)]
struct Floor {
    place: String,
    floor_bytes: u64,
}

impl<'a> Draft<'a> {
    fn for_bundle(bundle: &'a Bundle) -> Self {
        let names = bundle.sites.values().flat_map(|site| {
            let hired = site
                .chain
                .iter()
                .map(|candidate| candidate.provider.as_str());
            let inline = site
                .driver
                .iter()
                .flat_map(|drivers| drivers.keys().map(String::as_str));
            hired.chain(inline)
        });
        Self::new(names)
    }

    fn new(names: impl IntoIterator<Item = &'a str>) -> Self {
        let providers = names
            .into_iter()
            .map(|name| {
                (
                    name,
                    Ceiling {
                        ceiling: PLACEHOLDER_CEILING,
                    },
                )
            })
            .collect();
        Self {
            schema: CURRENT_HOST_SCHEMA,
            providers,
            floors: [Floor {
                place: Place::SeatScratch.to_string(),
                floor_bytes: SCRATCH_FLOOR_BYTES,
            }],
        }
    }
}

pub(in crate::verbs::setup) fn seed(path: &Path, bundle: &Bundle) -> anyhow::Result<Vec<String>> {
    Ok(seed_with(path, &Draft::for_bundle(bundle), publish)?)
}

fn seed_with(
    path: &Path,
    draft: &Draft<'_>,
    finish: impl FnOnce(NamedTempFile, &Path) -> io::Result<()>,
) -> Result<Vec<String>, UpgradeError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(kept(path)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            match create(path, draft, finish) {
                Ok(()) => Ok(placeholders(path, draft)),
                // A concurrent init (or an editor) has already claimed it.
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(kept(path)),
                Err(error) => Err(UpgradeError::Write(path.into(), error)),
            }
        }
        Err(error) => Err(read_error(path, error)),
    }
}

fn create(
    path: &Path,
    draft: &Draft<'_>,
    finish: impl FnOnce(NamedTempFile, &Path) -> io::Result<()>,
) -> io::Result<()> {
    let text = serde_json::to_string_pretty(draft).expect("the host draft serializes");
    fs::create_dir_all(parent(path))?;
    let staged = stage(path, &format!("{text}\n"))?;
    staged.as_file().sync_all()?;
    finish(staged, path)
}

fn publish(staged: NamedTempFile, path: &Path) -> io::Result<()> {
    staged
        .persist_noclobber(path)
        .map(drop)
        .map_err(|error| error.error)
}

fn kept(path: &Path) -> Vec<String> {
    let legacy = read(path).is_ok_and(|file| matches!(host_upgrade(path, &file.text), Ok(Some(_))));
    let hint = if legacy {
        format!(" Run `{UPGRADE_COMMAND}` to upgrade it.")
    } else {
        String::new()
    };
    vec![format!(
        "Left existing host file {} as it was.{hint}",
        path.display()
    )]
}

fn placeholders(path: &Path, draft: &Draft<'_>) -> Vec<String> {
    draft
        .providers
        .iter()
        .map(|(provider, policy)| {
            format!(
                "{}: providers.{provider}.ceiling = {} (placeholder to tune)",
                path.display(),
                policy.ceiling,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests;
