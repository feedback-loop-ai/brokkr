//! The steps of [`walk_files`](super::walk_files) (#288, slice G): the
//! listing of every entry under the scope, the skipped trees searched with
//! it, the keys the pinned entries are named by, the consumed key no entry
//! lists, and the digests under the one-entry rule. `walk_files` reads as
//! their sequence.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use brokkr_core::canonical::sha256_bytes;

use super::{
    at_stage, bounded_reference, listing, node, one_entry, unobservable, walk_key, CompileError,
    Entry, Found, ReadStage, Supplied,
};

/// The walk's listing of one layer: each directory to list, with how its
/// parent found it when it stands in a skipped tree; the entries it pins;
/// and the entries of skipped trees, which it never pins.
pub(super) struct Walk<'a> {
    dir: &'a Path,
    consumed: &'a BTreeMap<String, Supplied>,
    stack: Vec<(PathBuf, Option<Found>)>,
    paths: Vec<PathBuf>,
    unpinned: Vec<PathBuf>,
}

impl<'a> Walk<'a> {
    /// List every directory under `scope` in the layer `dir`, returning the
    /// entries to pin and the skipped trees' entries, which are not.
    pub(super) fn list(
        dir: &'a Path,
        scope: &Path,
        consumed: &'a BTreeMap<String, Supplied>,
    ) -> Result<(Vec<PathBuf>, Vec<PathBuf>), CompileError> {
        let mut walk = Walk {
            dir,
            consumed,
            stack: vec![(scope.to_path_buf(), None)],
            paths: Vec::new(),
            unpinned: Vec::new(),
        };
        while let Some((current, found)) = walk.stack.pop() {
            let in_skipped = found.is_some();
            let (handle, names) = listing(dir, &current, found, consumed)?;
            let handle = std::rc::Rc::new(handle);
            for name in names {
                walk.entry(&current, in_skipped, &handle, name)?;
            }
        }
        Ok((walk.paths, walk.unpinned))
    }

    /// The entry `name` that `current`, listed through `handle`, holds: a
    /// tree to descend, an entry to pin, or one of a skipped tree.
    fn entry(
        &mut self,
        current: &Path,
        in_skipped: bool,
        handle: &std::rc::Rc<std::fs::File>,
        name: OsString,
    ) -> Result<(), CompileError> {
        let (dir, consumed) = (self.dir, self.consumed);
        let path = current.join(&name);
        // A consumed entry is the one entry it was read by, never a tree
        // to descend (16-fix-e, return F1): whatever stands there now is
        // judged by its own check, which names who read it.
        let consumed_here = consumed.contains_key(&walk_key(dir, &path).unwrap_or_default());
        // A scaffold may also be the workspace from which Brokkr is
        // invoked. Its realm map and dialect library are workspace
        // declarations pinned into the RUN manifest, never bundle files:
        // changing either must not move the strategy's identity.
        let skipped = in_skipped
            || current == dir
                && (name.to_str()).is_some_and(|n| crate::places::unpinned_top_level(dir, n));
        if skipped && consumed.is_empty() {
            return Ok(());
        }
        if skipped {
            // Asked of the entry, never its target (16-fix-e, F3): a
            // linked directory here is one entry, not a tree. An entry
            // it cannot ask is refused as itself (second return F1).
            at_stage(ReadStage::Skipped, &path);
            let meta = std::fs::symlink_metadata(&path)
                .map_err(|error| unobservable(dir, &path, &error))?;
            if meta.is_dir() {
                let found = (std::rc::Rc::clone(handle), name, node(&meta));
                self.stack.push((path, Some(found)));
            } else {
                self.unpinned.push(path);
            }
        } else if !consumed_here && path.is_dir() {
            self.stack.push((path, None));
        } else if consumed_here || path.is_file() {
            refuse_secrets_store(&path)?;
            self.paths.push(path);
        }
        Ok(())
    }
}

/// A secrets store inside the bundle would ride the manifest digest:
/// rotation would change the digest AND the manifest would embed a SHA-256
/// of the secret file — an offline-guessing oracle (decision 0012, layer 2).
fn refuse_secrets_store(path: &Path) -> Result<(), CompileError> {
    if path.file_name().and_then(|n| n.to_str()) == Some("secrets.env") {
        return Err(CompileError::Invalid(format!(
            "bundle contains a secrets store '{}'; the store must \
             live outside the bundle dir (e.g. .forge/secrets.env) \
             so digests carry names only",
            path.display()
        )));
    }
    Ok(())
}

/// Each pinned path in sorted order with its manifest key, relative to the
/// layer `dir`.
pub(super) fn walk_keys(
    dir: &Path,
    mut paths: Vec<PathBuf>,
) -> Result<Vec<(PathBuf, String)>, CompileError> {
    paths.sort();
    let mut walked = Vec::with_capacity(paths.len());
    for path in paths {
        // Join actual components, never replace bytes inside a name:
        // Unix `scripts\gate.sh` is a different file from `scripts/gate.sh`.
        // Refuse names JSON cannot represent instead of losing bytes.
        let rel = path
            .strip_prefix(dir)
            .expect("walked under dir")
            .iter()
            .map(|name| {
                name.to_str().ok_or_else(|| {
                    CompileError::Invalid(format!(
                        "bundle filename '{}' is not UTF-8; manifest keys must preserve exact names",
                        path.display()
                    ))
                })
            })
            .collect::<Result<Vec<_>, _>>()?
            .join("/");
        // The composition and folder namespaces are computed, never read
        // from disk: a real file under one could otherwise forge provenance.
        if let Some(prefix) = super::compose::reserved_prefix(&rel) {
            return Err(CompileError::Invalid(format!(
                "bundle file '{rel}' uses the reserved '{prefix}' namespace; \
                 composition provenance is computed by the resolver, never \
                 supplied by a bundle"
            )));
        }
        walked.push((path, rel));
    }
    Ok(walked)
}

/// Refuse a consumed key the walk lists no entry for, naming who consumed
/// it.
pub(super) fn refuse_unlisted_consumed(
    consumed: &BTreeMap<String, Supplied>,
    walked: &[(PathBuf, String)],
) -> Result<(), CompileError> {
    if let Some((key, supplied)) = consumed
        .iter()
        .find(|(key, _)| !walked.iter().any(|(_, rel)| rel == *key))
    {
        return Err(CompileError::Invalid(format!(
            "{}, which the walk that pins the layer lists under no entry of the name {} it was \
             read by: the name reached the file through a case or normalization alias the \
             filesystem accepted, or the entry was removed after the read. A layer's identity \
             names a consumed file by the entry its directory lists, so it is refused; write the \
             reference as its directory lists it (decision 0065 slice one, design D7)",
            supplied.consumer,
            bounded_reference(key)
        )));
    }
    Ok(())
}

/// Each walked entry's digest, a consumed one's that of the buffer it was
/// read from, every pinned entry and then every unpinned one held to the
/// one-entry rule.
pub(super) fn walk_digests(
    dir: &Path,
    walked: Vec<(PathBuf, String)>,
    unpinned: Vec<PathBuf>,
    consumed: &BTreeMap<String, Supplied>,
) -> Result<BTreeMap<String, String>, CompileError> {
    let mut named: BTreeMap<(u64, u64), (Entry, String)> = BTreeMap::new();
    let mut files = BTreeMap::new();
    for (path, rel) in walked {
        let supplied = consumed.get(&rel);
        if supplied.is_none() {
            at_stage(ReadStage::Walked, &path);
        }
        one_entry(dir, &path, &rel, supplied, consumed, &mut named)?;
        let digest = match supplied {
            Some(supplied) => supplied.digest.clone(),
            None => sha256_bytes(&std::fs::read(&path)?),
        };
        files.insert(rel, digest);
    }
    // Judged after every pinned entry, so a consumed file's first entry is
    // the one it was read by; nothing here is read or pinned.
    for path in unpinned {
        let rel = path
            .strip_prefix(dir)
            .expect("walked under dir")
            .to_string_lossy();
        one_entry(dir, &path, &rel, None, consumed, &mut named)?;
    }
    Ok(files)
}
