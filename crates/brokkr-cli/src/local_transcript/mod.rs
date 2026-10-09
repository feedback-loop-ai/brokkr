//! The one local transcript reader (proposed decision 0055, D2) that
//! `brokkr transcript`, the terminal and the browser all consume: select
//! and validate a participant's reference, discover its one safe source,
//! bound-read it and project it, then mask its prose on the way out.
//!
//! It reads no environment. The local Claude projects home a legacy flat
//! id is synthesized against is the shell's to resolve, once per read, and
//! arrives here as an argument (#351).

use std::path::{Path, PathBuf};

use brokkr_view::transcript::{Snapshot, TranscriptRead, Unavailable};
use brokkr_view::Subject;

mod discover;
mod safe_fs;

use discover::acquisition_is_current;
pub(crate) use discover::{discover, source_size, Discovery};
#[cfg(test)]
pub(crate) use safe_fs::fault;

/// Select, validate, safely discover, bound-read and project `subject`'s
/// local transcript: the one read path every local surface consumes (D2).
/// `projects_home` is the local Claude projects root a legacy flat id is
/// synthesized against, or `None` when the shell has none.
pub fn read_local(subject: &Subject, projects_home: Option<&str>) -> TranscriptRead {
    let selection = brokkr_view::transcript::select_reference(
        subject.reference.as_ref(),
        subject.provenance,
        subject.legacy_id.as_deref(),
        projects_home,
    );
    let valid = match &selection.outcome {
        Ok(valid) => valid.clone(),
        Err(reason) => {
            return TranscriptRead::refused(
                selection.reference.clone(),
                selection.legacy,
                *reason,
                brokkr_view::transcript::explanation_for(*reason),
                None,
                false,
                0,
                0,
                None,
            )
        }
    };
    match discover(&valid) {
        Discovery::Refused(reason, explanation) => {
            let hint = brokkr_view::transcript::full_session(&valid, None);
            TranscriptRead::refused(
                selection.reference.clone(),
                selection.legacy,
                reason,
                explanation.unwrap_or_else(|| brokkr_view::transcript::explanation_for(reason)),
                None,
                false,
                0,
                0,
                hint,
            )
        }
        Discovery::Admitted(source) => {
            let source_identity = brokkr_view::transcript::SourceIdentity {
                device: source.identity.device,
                inode: source.identity.inode,
            };
            let hint = brokkr_view::transcript::full_session(&valid, Some(&source.path));
            // Re-derive the unique safe candidate through the retained
            // root and require the same recorded path and identity: a
            // swapped name, changed opening header or new ambiguity fails
            // closed rather than reading the old handle's bytes.
            if !acquisition_is_current(&valid, &source) {
                return refused_source(&selection, source.path, hint);
            }
            // Recheck the held leaf's checked lossless identity at the read
            // boundary. The handle is already the verified source, so this
            // never reopens a display path; a widening failure or a
            // mismatch is a bounded fail-closed `unreadable`.
            if source.file.identity().ok() != Some(source.identity) {
                return refused_source(&selection, source.path, hint);
            }
            let (bytes, overflow, eof) = match source
                .file
                .read_bounded(brokkr_view::transcript::SOURCE_CAP)
            {
                Ok(read) => read,
                Err(_) => return refused_source(&selection, source.path, hint),
            };
            let snapshot = Snapshot {
                bytes: &bytes,
                overflow,
                eof,
            };
            // `project` re-admits the snapshot; a UTF-8 failure becomes an
            // `unreadable` projection with zero counts and source-only
            // truncation, exactly as the failure-stage matrix fixes.
            let projection = brokkr_view::transcript::project(valid.kind, &snapshot);
            if let Some(reason) = projection.unavailable {
                let mut read = TranscriptRead::refused(
                    selection.reference.clone(),
                    selection.legacy,
                    reason,
                    brokkr_view::transcript::explanation_for(reason),
                    Some(source.path),
                    projection.truncated,
                    projection.skipped_lines,
                    projection.unrecognized_records,
                    hint,
                );
                read.source_identity = Some(source_identity);
                return read;
            }
            let mut read = TranscriptRead::readable(
                selection.reference.clone(),
                selection.legacy,
                valid.kind,
                Some(source.path),
                projection.turns,
                projection.truncated,
                projection.skipped_lines,
                projection.unrecognized_records,
            );
            read.source_identity = Some(source_identity);
            read
        }
    }
}

/// The operator's secrets store a read surface masks against: the one
/// beside the journal it reads, which is where the defaults put both
/// (`.forge/forge.db` and `.forge/secrets.env`).
pub(crate) fn store_beside(journal: &Path) -> PathBuf {
    journal.with_file_name("secrets.env")
}

/// Mask a read's prose against every value the store holds now (#380).
///
/// The harness writes its own session file, so a value the model echoed
/// is plaintext there; decision 0012's layer 5 is kept on the way out
/// instead, before any surface prints a block. The journal records no
/// seat's declared names, so every held value is a needle — a superset of
/// what any seat could have bound from this store. A value rotated or
/// removed since the run is no longer here and cannot be masked, which is
/// why the notice names what was masked against and says so. A store that
/// is absent, holds no values, or cannot be read masks nothing and says
/// so, naming the path, never the contents. A read with no turns has no
/// prose to mask.
pub(crate) fn mask_secrets(mut read: TranscriptRead, store: &Path) -> TranscriptRead {
    use brokkr_protocol::secret;
    if read.turns.is_empty() {
        return read;
    }
    let bindings = match secret::store_names(store)
        .and_then(|names| secret::resolve_bindings(store, &names))
    {
        Ok(bindings) => bindings,
        Err(error) => {
            read.notices.push(format!("secrets not masked: {error}"));
            return read;
        }
    };
    if bindings.is_empty() {
        read.notices.push(format!(
            "secrets not masked: no values found in {}; a value bound from another store, or \
             removed since the run, is shown as written",
            store.display()
        ));
        return read;
    }
    for block in read
        .turns
        .iter_mut()
        .flat_map(|turn| turn.blocks.iter_mut())
    {
        block.text = secret::mask_projected(&block.text, &bindings);
    }
    let names: Vec<&str> = bindings.iter().map(secret::BoundSecret::name).collect();
    read.notices.push(format!(
        "secrets masked against the store's current values for {}; a value rotated or \
         removed since the run is not masked",
        names.join(", ")
    ));
    read
}

/// The fail-closed read-boundary refusal: a fresh acquisition that no
/// longer matches the admitted source, a held-handle identity that cannot
/// be rechecked, or a bounded read that failed all present the same
/// `unreadable` result with the recorded path and hint.
fn refused_source(
    selection: &brokkr_view::transcript::Selection,
    path: String,
    hint: Option<String>,
) -> TranscriptRead {
    TranscriptRead::refused(
        selection.reference.clone(),
        selection.legacy,
        Unavailable::Unreadable,
        brokkr_view::transcript::explanation_for(Unavailable::Unreadable),
        Some(path),
        false,
        0,
        0,
        hint,
    )
}

#[cfg(test)]
pub(crate) mod fault_tests;
#[cfg(test)]
pub(crate) mod tests;
