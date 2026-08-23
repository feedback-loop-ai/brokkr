//! `forge` — the one shipped binary (decision 0003). No UI, no required
//! services: one executable, one workspace database.

mod doctor;
mod init;
mod ui;

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use forge_core::fold::{fold, RunState, Status};
use forge_runtime::{operator_command, Bundle, Engine};
use forge_store::Store;
use serde_json::{json, Value};

/// Exit codes: 0 completed/ok · 2 parked (operator needed) · 3 stopped ·
/// 1 error.
#[derive(Parser)]
#[command(name = "forge", version, about = "Deterministic delivery engine")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Scaffold a minimal reviewable bundle and prove it compiles.
    Init { dir: PathBuf },
    /// Serve the embedded read-only surface on loopback.
    Ui {
        #[arg(long, default_value = ".forge/forge.db")]
        db: PathBuf,
        #[arg(long, default_value_t = 8383)]
        port: u16,
        /// Open the system browser after binding.
        #[arg(long)]
        open: bool,
    },
    /// Verify tools, drivers, the workspace database, and optionally a
    /// bundle, without executing any agent.
    Doctor {
        #[arg(long)]
        bundle: Option<PathBuf>,
        #[arg(long, default_value = ".forge/forge.db")]
        db: PathBuf,
    },
    /// Validate a bundle and print its pinned manifest and digest.
    Compile {
        #[arg(long)]
        bundle: PathBuf,
    },
    /// Start a new run and drive it until it parks or finishes.
    Run {
        #[arg(long)]
        bundle: PathBuf,
        #[arg(long)]
        feature: String,
        #[arg(long, default_value = ".forge/forge.db")]
        db: PathBuf,
        #[arg(long)]
        repo: Option<PathBuf>,
    },
    /// Resume an existing run under its exact pinned bundle.
    Resume {
        #[arg(long)]
        bundle: PathBuf,
        #[arg(long)]
        run: String,
        #[arg(long, default_value = ".forge/forge.db")]
        db: PathBuf,
        #[arg(long)]
        repo: Option<PathBuf>,
    },
    /// Record an operator command (retry | stop) as journal events.
    Operator {
        #[arg(long)]
        run: String,
        /// "retry" re-runs the current phase; "stop" ends the run.
        command: String,
        #[arg(long)]
        reason: String,
        #[arg(long, default_value = ".forge/forge.db")]
        db: PathBuf,
    },
    /// Explain a run: status, phase, cursor, last ruling, blockers.
    Inspect {
        #[arg(long)]
        run: String,
        #[arg(long, default_value = ".forge/forge.db")]
        db: PathBuf,
    },
    /// Rebuild state from the journal twice and verify determinism.
    Replay {
        #[arg(long)]
        run: String,
        #[arg(long, default_value = ".forge/forge.db")]
        db: PathBuf,
    },
    /// Write the canonical NDJSON journal and pinned manifest.
    Export {
        #[arg(long)]
        run: String,
        #[arg(long, default_value = ".")]
        out: PathBuf,
        #[arg(long, default_value = ".forge/forge.db")]
        db: PathBuf,
    },
    /// Verify an exported journal offline: chain, envelopes, fold.
    VerifyRun { file: PathBuf },
    /// List runs in the workspace database.
    Runs {
        #[arg(long, default_value = ".forge/forge.db")]
        db: PathBuf,
    },
    /// (internal) Scripted forge-driver/v1 driver for machine proof.
    #[command(hide = true)]
    FakeDriver {
        #[arg(long)]
        script: PathBuf,
        #[arg(long)]
        state: PathBuf,
    },
}

fn status_str(status: &Status) -> &'static str {
    match status {
        Status::Running => "running",
        Status::AwaitingOperator => "awaiting_operator",
        Status::Completed => "completed",
        Status::Stopped => "stopped",
    }
}

fn summarize(state: &RunState) -> Value {
    json!({
        "run_id": state.run_id,
        "seq": state.seq,
        "status": status_str(&state.status),
        "phase": state.phase,
        "cursor": format!("{:?}", state.cursor),
        "park_reason": state.park_reason,
        "consecutive_failures": state.consecutive_failures,
        "last_decision": state.last_decision,
        "feature": state.feature,
    })
}

fn finish(state: &RunState) -> ExitCode {
    println!("{}", serde_json::to_string_pretty(&summarize(state)).unwrap());
    match state.status {
        Status::Completed => ExitCode::SUCCESS,
        Status::AwaitingOperator => ExitCode::from(2),
        Status::Stopped => ExitCode::from(3),
        Status::Running => ExitCode::from(1),
    }
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::from(1)
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode> {
    match cli.command {
        Cmd::Init { dir } => {
            let digest = init::init(&dir)?;
            eprintln!(
                "initialized reviewable bundle at {} (digest {digest})",
                dir.display()
            );
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Ui { db, port, open } => {
            ui::serve(db, port, open)?;
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Doctor { bundle, db } => {
            let report = doctor::doctor(bundle.as_deref(), &db);
            println!("{}", report.render());
            Ok(if report.healthy {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            })
        }
        Cmd::Compile { bundle } => {
            let bundle = Bundle::compile(&bundle)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "bundle": bundle.name,
                    "digest": bundle.manifest_digest(),
                    "phases": bundle.machine.phases,
                    "seats": bundle.seats.keys().collect::<Vec<_>>(),
                    "manifest": bundle.manifest,
                }))?
            );
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Run {
            bundle,
            feature,
            db,
            repo,
        } => {
            let bundle = Bundle::compile(&bundle)?;
            let store = Store::open(&db)?;
            let mut engine = Engine::start(store, bundle, &feature, repo)?;
            eprintln!("run started: {}", engine.run_id);
            let end = engine.drive()?;
            Ok(finish(&end.state))
        }
        Cmd::Resume {
            bundle,
            run,
            db,
            repo,
        } => {
            let bundle = Bundle::compile(&bundle)?;
            let store = Store::open(&db)?;
            let mut engine = Engine::resume(store, bundle, &run, repo)?;
            let end = engine.drive()?;
            Ok(finish(&end.state))
        }
        Cmd::Operator {
            run,
            command,
            reason,
            db,
        } => {
            anyhow::ensure!(
                command == "retry" || command == "stop",
                "operator command must be 'retry' or 'stop'"
            );
            let mut store = Store::open(&db)?;
            let operator = std::env::var("USER").unwrap_or_else(|_| "operator".into());
            operator_command(&mut store, &run, &command, &operator, &reason)?;
            eprintln!("recorded operator {command}; continue with: forge resume --run {run}");
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Inspect { run, db } => {
            let store = Store::open(&db)?;
            let state = fold(&store.load(&run)?)?;
            println!("{}", serde_json::to_string_pretty(&summarize(&state))?);
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Replay { run, db } => {
            let store = Store::open(&db)?;
            let events = store.load(&run)?;
            let first = format!("{:?}", fold(&events)?);
            let second = format!("{:?}", fold(&events)?);
            anyhow::ensure!(first == second, "replay was not deterministic");
            let state = fold(&events)?;
            println!("{}", serde_json::to_string_pretty(&json!({
                "events": events.len(),
                "chain": "verified",
                "replay": "deterministic",
                "state": summarize(&state),
            }))?);
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Export { run, out, db } => {
            let store = Store::open(&db)?;
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
            Ok(ExitCode::SUCCESS)
        }
        Cmd::VerifyRun { file } => {
            let ndjson = std::fs::read_to_string(&file)
                .with_context(|| format!("reading {}", file.display()))?;
            let state = forge_store::verify_export(&ndjson)?;
            println!("{}", serde_json::to_string_pretty(&json!({
                "chain": "verified",
                "state": summarize(&state),
            }))?);
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Runs { db } => {
            let store = Store::open(&db)?;
            for (run_id, feature, created_at) in store.list_runs()? {
                let state = fold(&store.load(&run_id)?)?;
                println!(
                    "{run_id}\t{feature}\t{created_at}\t{}\t{}",
                    status_str(&state.status),
                    state.phase.as_deref().unwrap_or("-")
                );
            }
            Ok(ExitCode::SUCCESS)
        }
        Cmd::FakeDriver { script, state } => {
            forge_protocol::fake::run_fake_driver(&script, &state)?;
            Ok(ExitCode::SUCCESS)
        }
    }
}
