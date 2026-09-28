//! The verbs that move a run's evidence across this machine's edge:
//! `anchor`, `export`, `import`, `verify-run` and the Looper `bridge`.

use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result};
use brokkr_store::Store;
use serde_json::{json, Value};

use crate::cli_args::{AnchorArgs, BridgeArgs, ExportArgs, ImportArgs, VerifyRunArgs};
use crate::{journal_of, manifest_beside, open_journal, render, selector, summarize, Access};

/// `brokkr anchor`: plant the journal head in refs/forge/<run>, or verify
/// the one planted with `--check`.
pub(crate) fn anchor(
    workspace: &Path,
    AnchorArgs {
        run,
        journal,
        repo,
        check,
    }: AnchorArgs,
) -> Result<ExitCode> {
    let store = Store::open(&journal.journal(workspace)?)?;
    let run = selector::resolve_run(&store, &run)?;
    if check {
        let report = brokkr_runtime::verify_anchor(&store, &repo, &run)?;
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        let sha = brokkr_runtime::anchor(&store, &repo, &run)?;
        eprintln!("anchored {run} at {sha}");
    }
    Ok(ExitCode::SUCCESS)
}

/// `brokkr export`: the canonical NDJSON journal and its pinned manifest,
/// and with `--redact` a marked, scrubbed copy of both.
pub(crate) fn export(
    workspace: &Path,
    ExportArgs {
        run,
        out,
        realms,
        db,
        redact,
    }: ExportArgs,
) -> Result<ExitCode> {
    let db = journal_of(workspace, realms, db)?;
    let store = open_journal(&db, Access::Read)?;
    let run = selector::resolve_run(&store, &run)?;
    std::fs::create_dir_all(&out)?;
    let ndjson = store.export_ndjson(&run)?;
    let manifest = store.manifest(&run)?;
    let journal_path = out.join(format!("{run}.ndjson"));
    std::fs::write(&journal_path, &ndjson)?;
    std::fs::write(
        out.join(format!("{run}.manifest.json")),
        serde_json::to_string_pretty(&manifest)?,
    )?;
    eprintln!("exported {}", journal_path.display());
    if redact {
        export_redacted(&out, &run, &ndjson, &manifest)?;
    }
    Ok(ExitCode::SUCCESS)
}

/// The `--redact` half of `export`, written beside the verbatim pair.
fn export_redacted(out: &Path, run: &str, ndjson: &str, manifest: &Value) -> Result<()> {
    // A sanitized copy that could pass as verbatim evidence
    // would be a forgery, so the copy is marked twice: in
    // its filenames and in its manifest, which also names
    // the consequence — redaction breaks the recorded event
    // hashes, and hash verification applies only to the
    // verbatim export.
    //
    // Journal and manifest are scrubbed through ONE
    // redaction, in that order: the manifest states the
    // bundle a run was invoked with and, in a mapped world,
    // the map file and the realm paths it named — operator-
    // machine detail the journal beside it is published to
    // withhold. Sharing the table also keeps `[path-1]`
    // naming one path across the pair.
    let raw_manifest = serde_json::to_string(manifest)?;
    let mut redactor = brokkr_store::Redactor::learn(&[ndjson, &raw_manifest]);
    let redacted = redactor.journal(ndjson)?;
    let redacted_path = out.join(format!("{run}.redacted.ndjson"));
    std::fs::write(&redacted_path, &redacted)?;
    let mut fields = redactor
        .document(manifest)
        .as_object()
        .cloned()
        .unwrap_or_default();
    fields.insert("redacted".into(), json!(true));
    fields.insert(
        "redaction".into(),
        json!({
            "scheme": "absolute filesystem paths — POSIX, drive-letter, \
                       and UNC — in event payload string fields, and in \
                       this manifest, rewritten to stable placeholders \
                       ([path-N]), usernames to [user-N]; scheme URLs \
                       survive as a declared bound",
            "hashes": "recorded event hashes predate redaction and no \
                       longer match; a pinned realms map's sha256 is \
                       likewise the digest of the map as it was, not of \
                       the scrubbed copy printed here; hash verification \
                       applies only to the verbatim export",
        }),
    );
    std::fs::write(
        out.join(format!("{run}.redacted.manifest.json")),
        serde_json::to_string_pretty(&Value::Object(fields))?,
    )?;
    eprintln!("exported {} (redacted)", redacted_path.display());
    Ok(())
}

/// `brokkr import`: adopt an exported run byte-identically.
pub(crate) fn import(
    workspace: &Path,
    ImportArgs { from, realms, db }: ImportArgs,
) -> Result<ExitCode> {
    let db = journal_of(workspace, realms, db)?;
    // The sidecar is required, not optional: it is where an
    // export declares itself redacted, and an import that
    // shrugged at a missing manifest would accept exactly the
    // pair whose declaration went missing.
    let manifest_path = manifest_beside(&from);
    let ndjson = std::fs::read_to_string(&from).context(format!("reading {}", from.display()))?;
    let raw = std::fs::read_to_string(&manifest_path)
        .context(format!("reading {}", manifest_path.display()))?;
    let manifest: Value =
        serde_json::from_str(&raw).context(format!("parsing {}", manifest_path.display()))?;
    let mut store = open_journal(&db, Access::Append)?;
    let adoption = store.import_run(&ndjson, &manifest, &from)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "run_id": adoption.run_id,
            "events": adoption.events,
            "chain": "verified",
            "adopted": "byte-identical",
            "journal_head_hash": adoption.head_hash,
            "imported_at": adoption.arrival.imported_at,
            "imported_from": adoption.arrival.imported_from,
        }))?
    );
    // `import_run`'s run_id gate already refuses anything a
    // terminal could not print plainly, so this is belt over
    // braces — but the house rule is that a journal string
    // reaching a tty goes through `Safe`, and the one line
    // telling an operator the adoption happened is a poor place
    // to start making exceptions.
    eprintln!(
        "imported {} into {} ({} events)",
        render::Safe::new(&adoption.run_id).as_str(),
        db.display(),
        adoption.events
    );
    Ok(ExitCode::SUCCESS)
}

/// `brokkr verify-run`: an exported journal's chain, envelopes and fold.
pub(crate) fn verify_run(VerifyRunArgs { file }: VerifyRunArgs) -> Result<ExitCode> {
    let ndjson = std::fs::read_to_string(&file).context(format!("reading {}", file.display()))?;
    let state = brokkr_store::verify_export(&ndjson)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "chain": "verified",
            "state": summarize(&state),
        }))?
    );
    Ok(ExitCode::SUCCESS)
}

/// `brokkr bridge`: synchronize a Looper-bound run, once or following,
/// for at most `bridge_iteration_limit` rounds when a test bounds it.
pub(crate) fn bridge(
    workspace: &Path,
    BridgeArgs {
        run,
        journal,
        looper_url,
        token_env,
        follow,
        interval_ms,
    }: BridgeArgs,
    bridge_iteration_limit: Option<usize>,
) -> Result<ExitCode> {
    let db = journal.journal(workspace)?;
    let token = std::env::var(&token_env)
        .with_context(|| format!("reading producer credential from {token_env}"))?;
    anyhow::ensure!(!token.trim().is_empty(), "producer credential is empty");
    let transport = brokkr_bridge::HttpTransport::new(looper_url, token);
    let mut bridge = brokkr_bridge::Bridge::new(transport);
    let mut command_cursor = 0;
    let mut iteration = 0usize;
    loop {
        let mut store = open_journal(&db, Access::Append)?;
        let report = bridge.sync_once(
            &mut store,
            &run,
            time::OffsetDateTime::now_utc(),
            command_cursor,
        )?;
        command_cursor = report.last_command_cursor;
        println!(
            "{}",
            serde_json::to_string(&json!({
                "run_id": run,
                "registered": report.registered,
                "submitted": report.submitted,
                "replayed": report.replayed,
                "commands": report.commands,
                "last_forge_sequence": report.last_forge_sequence,
            }))?
        );
        iteration += 1;
        if !follow || bridge_iteration_limit.is_some_and(|limit| iteration >= limit) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(interval_ms.max(100)));
    }
    Ok(ExitCode::SUCCESS)
}
