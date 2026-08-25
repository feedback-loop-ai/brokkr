//! `forge` — the one shipped binary (decision 0003). No UI, no required
//! services: one executable, one workspace database.

mod doctor;
mod init;
mod recipes;
mod ui;

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{ArgGroup, Parser, Subcommand};
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
    /// Per-seat cost and session accounting from journal checkpoints —
    /// the LaneTally join surface (stable seat ids, journal-derived).
    Costs {
        #[arg(long)]
        run: String,
        #[arg(long, default_value = ".forge/forge.db")]
        db: PathBuf,
    },
    /// Anchor a run's journal head in refs/forge/<run> (tamper evidence),
    /// or verify the existing anchor with --check.
    Anchor {
        #[arg(long)]
        run: String,
        #[arg(long, default_value = ".forge/forge.db")]
        db: PathBuf,
        #[arg(long, default_value = ".")]
        repo: PathBuf,
        /// Verify instead of writing a new anchor.
        #[arg(long)]
        check: bool,
    },
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
    #[command(group(ArgGroup::new("delivery").required(true).args(["bundle", "recipe"])))]
    Run {
        #[arg(long)]
        bundle: Option<PathBuf>,
        /// Named recipe, resolved to <recipes-dir>/<name>.
        #[arg(long)]
        recipe: Option<String>,
        #[arg(long, default_value = "recipes")]
        recipes_dir: PathBuf,
        #[arg(long)]
        feature: String,
        #[arg(long, default_value = ".forge/forge.db")]
        db: PathBuf,
        #[arg(long)]
        repo: Option<PathBuf>,
    },
    /// Resume an existing run under its exact pinned bundle.
    #[command(group(ArgGroup::new("delivery").required(true).args(["bundle", "recipe"])))]
    Resume {
        #[arg(long)]
        bundle: Option<PathBuf>,
        /// Named recipe, resolved to <recipes-dir>/<name>.
        #[arg(long)]
        recipe: Option<String>,
        #[arg(long, default_value = "recipes")]
        recipes_dir: PathBuf,
        #[arg(long)]
        run: String,
        #[arg(long, default_value = ".forge/forge.db")]
        db: PathBuf,
        #[arg(long)]
        repo: Option<PathBuf>,
    },
    /// Re-run a past run's feature as a NEW run under another bundle or
    /// recipe, so outcomes can be compared by run id. No stored linkage.
    #[command(group(ArgGroup::new("delivery").required(true).args(["bundle", "recipe"])))]
    Rerun {
        /// The source run whose feature is re-run.
        #[arg(long)]
        run: String,
        #[arg(long)]
        bundle: Option<PathBuf>,
        /// Named recipe, resolved to <recipes-dir>/<name>.
        #[arg(long)]
        recipe: Option<String>,
        #[arg(long, default_value = "recipes")]
        recipes_dir: PathBuf,
        #[arg(long, default_value = ".forge/forge.db")]
        db: PathBuf,
        #[arg(long)]
        repo: Option<PathBuf>,
    },
    /// The recipe library: bundle directories as named, swappable
    /// delivery strategies.
    Recipes {
        #[command(subcommand)]
        command: RecipesCmd,
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
    /// Run a built-in forge-driver/v1 adapter (claude | codex | exec).
    /// Bundles reference these as {forge} driver <kind> -- <extra args>.
    Driver {
        kind: String,
        /// Arguments after -- pass to the agent CLI (claude/codex) or
        /// form the command template (exec).
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
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

#[derive(Subcommand)]
enum RecipesCmd {
    /// List recipes under --dir plus the built-in bundles; broken ones
    /// print a warning line, never abort the listing.
    List {
        #[arg(long, default_value = "recipes")]
        dir: PathBuf,
    },
    /// Install a recipe from a local path or a git URL into <dir>/<name>.
    Add {
        source: String,
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "recipes")]
        dir: PathBuf,
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
        Cmd::Costs { run, db } => {
            let store = Store::open(&db)?;
            let events = store.load(&run)?;
            let mut effect_seat: std::collections::BTreeMap<String, String> =
                std::collections::BTreeMap::new();
            let mut seats: std::collections::BTreeMap<String, (u64, u64, f64)> =
                std::collections::BTreeMap::new();
            for event in &events {
                let payload = &event.payload;
                match event.event_type {
                    forge_core::EventType::EffectRequested => {
                        if let (Some(id), Some(seat)) = (
                            payload.get("effect_id").and_then(Value::as_str),
                            payload.get("seat").and_then(Value::as_str),
                        ) {
                            effect_seat.insert(id.to_string(), seat.to_string());
                        }
                    }
                    forge_core::EventType::EffectStarted => {
                        if let Some(seat) = payload
                            .get("effect_id")
                            .and_then(Value::as_str)
                            .and_then(|id| effect_seat.get(id))
                        {
                            seats.entry(seat.clone()).or_default().0 += 1;
                        }
                    }
                    forge_core::EventType::EffectCheckpointed => {
                        let seat = payload
                            .get("effect_id")
                            .and_then(Value::as_str)
                            .and_then(|id| effect_seat.get(id))
                            .cloned();
                        if let Some(seat) = seat {
                            let checkpoint = &payload["checkpoint"];
                            let entry = seats.entry(seat).or_default();
                            entry.1 += checkpoint.get("num_turns").and_then(Value::as_u64).unwrap_or(0);
                            entry.2 += checkpoint
                                .get("total_cost_usd")
                                .and_then(Value::as_f64)
                                .unwrap_or(0.0);
                        }
                    }
                    _ => {}
                }
            }
            let total: f64 = seats.values().map(|(_, _, c)| c).sum();
            let report: serde_json::Map<String, Value> = seats
                .into_iter()
                .map(|(seat, (attempts, turns, cost))| {
                    (seat, json!({"attempts": attempts, "turns": turns, "cost_usd": cost}))
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&json!({
                "run_id": run,
                "seats": report,
                "total_cost_usd": total,
            }))?);
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Anchor { run, db, repo, check } => {
            let store = Store::open(&db)?;
            if check {
                let report = forge_runtime::verify_anchor(&store, &repo, &run)?;
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                let sha = forge_runtime::anchor(&store, &repo, &run)?;
                eprintln!("anchored {run} at {sha}");
            }
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
            recipe,
            recipes_dir,
            feature,
            db,
            repo,
        } => {
            let bundle = Bundle::compile(&recipes::resolve(bundle, recipe, &recipes_dir)?)?;
            let store = Store::open(&db)?;
            let mut engine = Engine::start(store, bundle, &feature, repo)?;
            eprintln!("run started: {}", engine.run_id);
            let end = engine.drive()?;
            Ok(finish(&end.state))
        }
        Cmd::Resume {
            bundle,
            recipe,
            recipes_dir,
            run,
            db,
            repo,
        } => {
            let bundle = Bundle::compile(&recipes::resolve(bundle, recipe, &recipes_dir)?)?;
            let store = Store::open(&db)?;
            let mut engine = Engine::resume(store, bundle, &run, repo)?;
            let end = engine.drive()?;
            Ok(finish(&end.state))
        }
        Cmd::Rerun {
            run,
            bundle,
            recipe,
            recipes_dir,
            db,
            repo,
        } => {
            let store = Store::open(&db)?;
            let events = store
                .load(&run)
                .with_context(|| format!("loading source run '{run}'"))?;
            let feature = events
                .first()
                .filter(|e| e.event_type == forge_core::EventType::RunStarted)
                .and_then(|e| e.payload.get("feature"))
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    anyhow::anyhow!("source run '{run}' has no run/started feature to re-run")
                })?
                .to_string();
            let bundle = Bundle::compile(&recipes::resolve(bundle, recipe, &recipes_dir)?)?;
            let mut engine = Engine::start(store, bundle, &feature, repo)?;
            eprintln!(
                "rerun of {run} as {} under {}",
                engine.run_id, engine.bundle.name
            );
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
        Cmd::Driver { kind, args } => {
            let kind = forge_protocol::adapters::AdapterKind::parse(&kind)
                .ok_or_else(|| anyhow::anyhow!("unknown driver '{kind}'; known: claude, codex, exec"))?;
            let extra = args.iter().position(|a| a == "--")
                .map(|i| args[i + 1..].to_vec())
                .unwrap_or(args);
            forge_protocol::adapters::serve(kind, extra)?;
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Recipes { command } => {
            match command {
                RecipesCmd::List { dir } => recipes::list(&dir)?,
                RecipesCmd::Add { source, name, dir } => recipes::add(&source, &name, &dir)?,
            }
            Ok(ExitCode::SUCCESS)
        }
        Cmd::FakeDriver { script, state } => {
            forge_protocol::fake::run_fake_driver(&script, &state)?;
            Ok(ExitCode::SUCCESS)
        }
    }
}
