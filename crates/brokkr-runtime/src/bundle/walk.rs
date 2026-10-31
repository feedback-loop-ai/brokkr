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
    at_stage, bounded_reference, folded, listing, node, one_entry, unobservable, walk_key,
    CompileError, Entry, Found, ReadStage, Supplied,
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
        // changing either must not move the strategy's identity. Git's
        // metadata and a path git reports ignored are the same kind of
        // answer for a bundle that is a repository's root (#908): the one
        // full-relative-path decision is asked here, and a skipped tree's
        // children inherit it, so asking git is refused by name when it
        // cannot answer.
        let skipped = if in_skipped {
            true
        } else {
            let rel = path.strip_prefix(dir).expect("an entry under the layer");
            crate::places::unpinned_reason(dir, rel)
                .map_err(|refused| CompileError::Invalid(refused.to_string()))?
                .is_some()
        };
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

/// The most pinned files and pinned bytes one bundle's walk may hold
/// (#908). A bundle past either is refused before any run starts, so no
/// `run/started` manifest carries it and no reader of the run pays for it.
const MAX_FILES: usize = 5_000;
pub(super) const MAX_BYTES: u64 = 64 * 1024 * 1024;

/// Refuse a layer whose pinned walk is past [`MAX_FILES`] files or
/// [`MAX_BYTES`] bytes (#908), naming the bundle, the count or bytes, and
/// the largest top-level trees. Judged from the walked paths before any
/// byte is read, so a bundle this large is refused without hashing it.
pub(super) fn refuse_oversized(
    dir: &Path,
    walked: &[(PathBuf, String)],
) -> Result<(), CompileError> {
    let mut total = 0u64;
    let mut trees: BTreeMap<&str, (bool, usize, u64)> = BTreeMap::new();
    for (path, rel) in walked {
        let bytes = std::fs::metadata(path)?.len();
        total = total.saturating_add(bytes);
        tally(&mut trees, rel, bytes);
    }
    let files_past = walked.len() > MAX_FILES;
    if !files_past && total <= MAX_BYTES {
        return Ok(());
    }
    Err(past_bound(dir, files_past, total, walked.len(), trees))
}

/// The refusal of a bundle past a bound (#908): `files_past` says which
/// bound was passed, `total` the bytes and `count` the files measured, and
/// `trees` each top-level name's file count and bytes. One home for the
/// preflight's refusal and the refusal of a read that passes the byte
/// budget after it (fix round 908d), so the two cannot drift.
fn past_bound(
    dir: &Path,
    files_past: bool,
    total: u64,
    count: usize,
    trees: BTreeMap<&str, (bool, usize, u64)>,
) -> CompileError {
    let mut ranked: Vec<(&str, (bool, usize, u64))> = trees.into_iter().collect();
    if files_past {
        ranked.sort_by(|(an, (_, ac, _)), (bn, (_, bc, _))| bc.cmp(ac).then_with(|| an.cmp(bn)));
    } else {
        ranked.sort_by(|(an, (_, _, ab)), (bn, (_, _, bb))| bb.cmp(ab).then_with(|| an.cmp(bn)));
    }
    let largest: Vec<String> = ranked
        .iter()
        .take(3)
        .map(|(name, (tree, count, bytes))| {
            let shown = if *tree {
                format!("{name}/")
            } else {
                (*name).to_string()
            };
            match files_past {
                true => format!("{shown} ({count} files)"),
                false => format!("{shown} ({bytes} bytes)"),
            }
        })
        .collect();
    let measured = match files_past {
        true => format!("pins {count} files, past the bound of {MAX_FILES} files"),
        false => format!("pins {total} bytes, past the bound of {MAX_BYTES} bytes (64 MiB)"),
    };
    CompileError::Invalid(format!(
        "the bundle at {} {measured} (#908): the largest top-level trees are {}; it is refused \
         before any run starts, so no run carries a manifest this large (decision 0083 \
         amendment)",
        dir.display(),
        largest.join(", ")
    ))
}

/// The refusal of a bound read that passes [`MAX_BYTES`] (#908, fix round
/// 908e): the same one [`past_bound`] text, charged with the `total` bytes
/// the read took and the entry `rel` it read, never a second wording.
fn past_read_bound(dir: &Path, rel: &str, total: u64) -> CompileError {
    let mut trees = BTreeMap::new();
    tally(&mut trees, rel, total);
    past_bound(dir, false, total, 0, trees)
}

/// Refuse the buffer a bound read of `rel` took when it passed [`MAX_BYTES`],
/// with [`past_read_bound`]'s text, so the read's caller refuses by name
/// rather than admit bytes past the bound (#908, fix round 908e).
pub(super) fn refuse_read_past_bound(
    dir: &Path,
    rel: &str,
    bytes: &[u8],
) -> Result<(), super::InputFault> {
    if bytes.len() as u64 <= MAX_BYTES {
        return Ok(());
    }
    let refusal = past_read_bound(dir, rel, bytes.len() as u64);
    Err(super::Place::of(
        super::FaultKind::Unbound,
        refusal.to_string(),
    ))
}

/// The top-level name an ACTIVE input stands under, where that name is one
/// the walk skips (decision 0066 ruling 5, correcting decision 0065's
/// design D7). A seat's charter decides what the seat is told and a layer's
/// table decides how the run is ruled, so neither may sit where the
/// manifest's file map does not look: the skip is sound for operator
/// configuration only while nothing a recipe EXECUTES is read from under
/// it. One predicate, shared with the walk, so the two cannot drift.
///
/// `root` is the declaring layer's canonical directory and `reference` is
/// what that layer wrote. Judged twice: here, the reference as written, its
/// `.` and `..` folded without touching the disk, which catches a path
/// spelled into the skipped tree — a link there that points back out
/// included, since the link is itself bytes nobody pins; and in
/// [`bound_input`](super::bound_input), its canonical target, which catches
/// an allowed spelling whose file, or whose parent, is a link into it. That
/// second question is asked of the one resolution the read is then bound
/// to.
///
/// A reference written OUT of the layer altogether — `../shared/role.md`
/// — escapes the same map the same way, and no other route pins it: an
/// agent's charter is pinned by its library record, a dialect's
/// instructions by the dialect pin, and an inline role by the file map or
/// by nothing. It is refused on the same terms. The answer is WHERE the
/// input stands, as the clause both refusals carry.
///
/// Second council H5: A `..` STEP IS NOT A PATH THE WALK EVER TAKES. The
/// walk descends real directory entries — through a link, though a
/// consumed input whose link leaves the layer is refused by
/// [`bound_input`](super::bound_input) under operator ruling 3 — and every
/// key it writes is a chain of such entries. A
/// reference that walks back UP cannot be one of those keys: with
/// `base/alias -> ../outside/child`, `alias/../charter.md` folds
/// lexically to `base/charter.md`, which the map does pin, while the file
/// a reader opens is `outside/charter.md`, which nothing pins. The chief
/// edited that file and every manifest digest stayed the same.
///
/// So the fold is no longer asked to stand in for the filesystem: a
/// reference that reaches its file through `..` is refused outright, and
/// what remains is exactly the set of paths the walk enumerates.
pub(super) fn unpinned_active_input(
    root: &Path,
    reference: &str,
) -> Result<Option<String>, crate::places::SkipError> {
    let folded = folded(&root.join(reference));
    if !folded.starts_with(root) {
        return Ok(Some(
            "which stands outside the layer's own directory, where the bundle's file walk \
             never reaches"
                .to_string(),
        ));
    }
    // The narrower question first, so a reference spelled into operator
    // configuration keeps naming the tree it reached.
    if let Some(place) = skipped_path(root, &folded)? {
        return Ok(Some(place));
    }
    Ok(Path::new(reference)
        .components()
        .any(|part| part == std::path::Component::ParentDir)
        .then(|| {
            "which reaches its file through a '..' step — never a path the bundle's file \
             walk takes, so a link earlier in it can put the file a reader opens outside \
             everything the walk pinned"
                .to_string()
        }))
}

/// The clause for `path`, under the declaring layer's canonical `root`,
/// when the one full-relative-path skip decision ([`unpinned_reason`])
/// skips it. A fixed top-level name names itself; a git-ignored path is
/// named by its full relative path, so a file ignored under a tree at any
/// depth reads as the place it is (#908).
pub(super) fn skipped_path(
    root: &Path,
    path: &Path,
) -> Result<Option<String>, crate::places::SkipError> {
    let Ok(rel) = path.strip_prefix(root) else {
        return Ok(None);
    };
    let Some(skip) = crate::places::unpinned_reason(root, rel)? else {
        return Ok(None);
    };
    let holds = crate::places::unpinned_holds(skip);
    let (key, preposition, what) = match skip {
        crate::places::Skip::Workspace
        | crate::places::Skip::State
        | crate::places::Skip::Shelf
        | crate::places::Skip::GitMetadata => (first_component(rel), "under", "a top-level name"),
        crate::places::Skip::GitIgnored => (relative_key(rel), "at", "a path"),
    };
    Ok(Some(format!(
        "which stands {preposition} '{key}' — {what} the bundle's file walk does not pin, \
         because it holds {holds}"
    )))
}

/// The first component of `rel`, as the name a fixed top-level skip is
/// written by.
fn first_component(rel: &Path) -> String {
    rel.components()
        .next()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// `rel` with each component joined by `/`, as the walk writes its keys.
fn relative_key(rel: &Path) -> String {
    rel.components()
        .map(|part| part.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
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
    let mut read = 0u64;
    let mut read_trees: BTreeMap<&str, (bool, usize, u64)> = BTreeMap::new();
    for (path, rel) in &walked {
        let supplied = consumed.get(rel);
        if supplied.is_none() {
            at_stage(ReadStage::Walked, path);
        }
        one_entry(dir, path, rel, supplied, consumed, &mut named)?;
        let digest = match supplied {
            Some(supplied) => {
                // A consumed file's bytes were read by the bound read that
                // supplied its digest; its entry charges the length of that
                // buffer, never the entry's metadata, so a file shrunk after
                // the read cannot pay less than the bytes its digest pins
                // (#908, fix rounds 908d and 908e).
                spend(
                    dir,
                    &walked,
                    rel,
                    supplied.bytes,
                    &mut read,
                    &mut read_trees,
                )?;
                supplied.digest.clone()
            }
            None => {
                // At most the budget still left plus one byte: a file that
                // reaches the extra byte has outgrown the bound after the
                // metadata preflight, and is refused rather than hashed
                // (#908, fix round 908d).
                let bytes = read_file_within(path, MAX_BYTES - read)?;
                let spent = bytes.len() as u64;
                spend(dir, &walked, rel, spent, &mut read, &mut read_trees)?;
                sha256_bytes(&bytes)
            }
        };
        files.insert(rel.clone(), digest);
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

/// Add one walked entry's `bytes` under its top-level name in `trees`, for
/// the refusal's ranking: a `/` in `rel` marks a tree, and each name's file
/// count and bytes accumulate.
fn tally<'a>(trees: &mut BTreeMap<&'a str, (bool, usize, u64)>, rel: &'a str, bytes: u64) {
    let (top, tree) = match rel.split_once('/') {
        Some((top, _)) => (top, true),
        None => (rel, false),
    };
    let held = trees.entry(top).or_insert((tree, 0, 0));
    held.0 |= tree;
    held.1 += 1;
    held.2 = held.2.saturating_add(bytes);
}

/// Count `bytes` for the walked entry `rel` against the walk's byte budget
/// (#908, fix round 908d), recording its top-level tree for the refusal,
/// and refuse the bundle the moment the cumulative bytes pass
/// [`MAX_BYTES`]. `read` is what earlier entries spent; it is never past the
/// bound on entry, and [`read_within`] caps what one read can add.
fn spend<'a>(
    dir: &Path,
    walked: &'a [(PathBuf, String)],
    rel: &'a str,
    bytes: u64,
    read: &mut u64,
    trees: &mut BTreeMap<&'a str, (bool, usize, u64)>,
) -> Result<(), CompileError> {
    *read = read.saturating_add(bytes);
    tally(trees, rel, bytes);
    if *read > MAX_BYTES {
        return Err(past_bound(
            dir,
            false,
            *read,
            walked.len(),
            std::mem::take(trees),
        ));
    }
    Ok(())
}

/// At most `left` bytes plus one through `reader`: every read of bundle
/// bytes takes this one cap, so none is unbounded (#908, fix round 908e).
/// The extra byte is the witness that the bytes outgrew the bound.
pub(super) fn read_within<R: std::io::Read>(reader: R, left: u64) -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    let mut bytes = Vec::new();
    reader.take(left + 1).read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// The bytes of `path`, at most `left` plus one: the walk never reads, and
/// so never hashes, past the byte budget still left in it (#908, fix round
/// 908d). The extra byte is the witness that the file outgrew the bound
/// after [`refuse_oversized`] measured the walked paths.
fn read_file_within(path: &Path, left: u64) -> Result<Vec<u8>, CompileError> {
    Ok(read_within(std::fs::File::open(path)?, left)?)
}
