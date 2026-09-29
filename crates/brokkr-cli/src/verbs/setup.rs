//! The verbs that prepare the workspace, its libraries and its drivers:
//! `init`, `doctor`, `compile`, `recipes`, `agents`, `muninn`, `secrets`,
//! `driver` and the hidden `fake-driver`.

use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result};
use brokkr_runtime::realms::World;

use crate::cli_args::{CompileArgs, DoctorArgs, DriverArgs, FakeDriverArgs, InitArgs};
use crate::{agents, doctor, init, muninn, recipes};
use crate::{compile_in, compile_in_realm, compiled_view, driver_payload, now_rfc3339};
use crate::{world_and_hearths, AgentsCmd, Exit, MuninnCmd, RecipesCmd, SecretsCmd};

/// `brokkr init`: scaffold a reviewable bundle for the workspace.
pub(crate) fn init(workspace: &Path, InitArgs { dir }: InitArgs) -> Result<ExitCode> {
    // The recipe lands in `dir`; the repository it describes is
    // the WORKSPACE, read for its manifests so the implement and
    // verify seats are told commands that would actually run
    // there. Same tree every other verb resolves (decision 0023),
    // for the same reason: what a command produces is a function
    // of its arguments, not of where the caller happens to stand.
    let path = std::env::var_os("PATH").unwrap_or_default();
    let scaffold = init::init(&dir, workspace, &path, std::env::consts::OS)?;
    eprintln!(
        "initialized reviewable bundle at {} (digest {})",
        dir.display(),
        scaffold.digest
    );
    // Which agent CLI the seats are hired from, and which boundary
    // the realm declares — the two facts read off this machine.
    for note in &scaffold.notes {
        eprintln!("{note}");
    }
    // The scaffold carries its own `adapters/` and `agents/`,
    // where the trust tier its gate seats compile against and the
    // tool grants its seats run under are declared (decisions
    // 0021 and 0016). Every other verb reads those trees from the
    // workspace, which is the directory brokkr is run in — so say
    // once, here, where to stand.
    eprintln!(
        "run brokkr from inside {} — its adapters/ and agents/ declare \
         the trust tier and the tool grants its seats run under",
        dir.display()
    );
    // Decision 0046: the scaffolded seats run under the realm's
    // boundary, and `namespace` — the default — is the one that
    // needs bubblewrap; a realm may declare `harness` instead, and
    // a scaffold that already did asks nothing of bubblewrap.
    if let (true, Err(reason)) = (scaffold.namespace, brokkr_protocol::hands::bwrap_on(&path)) {
        eprintln!(
            "warning: {reason}; the scaffolded seats [\"ship\", \"verify\"] \
             declare hands and run under the realm's boundary — `namespace`, \
             the default, needs bubblewrap on PATH, and a realm may declare \
             `harness` instead (decision 0046)"
        );
    }
    Ok(Exit::Completed.into())
}

/// `brokkr doctor`: tools, drivers, the database and optionally a bundle.
pub(crate) fn doctor(
    DoctorArgs {
        bundle,
        realms,
        db,
        secrets_file,
    }: DoctorArgs,
) -> Result<ExitCode> {
    let report = doctor::doctor(
        bundle.as_deref(),
        db.as_deref(),
        &secrets_file,
        realms.as_deref(),
    );
    println!("{}", report.render());
    Ok(if report.healthy {
        Exit::Completed.into()
    } else {
        Exit::Failed.into()
    })
}

/// `brokkr compile`: validate a bundle and print its pinned manifest.
pub(crate) fn compile(workspace: &Path, CompileArgs { bundle }: CompileArgs) -> Result<ExitCode> {
    let world = World::discover(workspace, None)?;
    let bundle = compile_in_realm(workspace, &bundle, world.as_ref(), workspace)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&compiled_view(&bundle, world.as_ref()))?
    );
    Ok(Exit::Completed.into())
}

/// `brokkr recipes`: list, add or show a recipe.
pub(crate) fn recipes(workspace: &Path, command: RecipesCmd) -> Result<ExitCode> {
    match command {
        RecipesCmd::List { dir } => recipes::list(workspace, &dir)?,
        RecipesCmd::Add { source, name, dir } => recipes::add(workspace, &source, &name, &dir)?,
        RecipesCmd::Show { name, dir } => {
            let world = World::discover(workspace, None)?;
            let recipe = recipes::resolve(None, Some(name), &dir)?;
            let bundle = compile_in(workspace, &recipe, world.as_ref())?;
            println!(
                "{}",
                serde_json::to_string_pretty(&compiled_view(&bundle, None))?
            );
        }
    }
    Ok(Exit::Completed.into())
}

/// `brokkr agents`: list the agent library or show one definition.
pub(crate) fn agents(command: AgentsCmd) -> Result<ExitCode> {
    match command {
        AgentsCmd::List { agents_dir } => agents::list(&agents_dir)?,
        AgentsCmd::Show {
            name,
            agents_dir,
            adapters_dir,
        } => agents::show(&name, &agents_dir, &adapters_dir)?,
    }
    Ok(Exit::Completed.into())
}

/// `brokkr muninn`: read the fleet and record proposals, or list them.
pub(crate) fn muninn(workspace: &Path, command: MuninnCmd) -> Result<ExitCode> {
    match command {
        MuninnCmd::Run {
            realms,
            db,
            agents_dir,
            adapters_dir,
            record,
        } => {
            // `world_and_hearths`, not `hearths_of`: the raven reports
            // a moved crossing as a finding, so it reads the world
            // with `inspect` and keeps the resolved map beside the
            // journals it names (decision 0057).
            let (world, hearths) = world_and_hearths(workspace, realms, db)?;
            muninn::run(
                &hearths,
                world.as_ref(),
                &agents_dir,
                &adapters_dir,
                &record,
                &now_rfc3339(),
            )
        }
        MuninnCmd::List { record, json } => {
            muninn::list(&record, json)?;
            Ok(Exit::Completed.into())
        }
    }
}

/// `brokkr secrets`: set, list or remove a name in the secrets store.
pub(crate) fn secrets(command: SecretsCmd) -> Result<ExitCode> {
    use brokkr_protocol::secret;
    match command {
        SecretsCmd::Set { name, secrets_file } => {
            let value = std::io::read_to_string(std::io::stdin())
                .context("reading the secret value from stdin")?;
            // stdin values arrive newline-terminated; the value
            // itself must be single-line (validated in the store).
            let value = value.strip_suffix('\n').unwrap_or(&value);
            let value = value.strip_suffix('\r').unwrap_or(value);
            let warning =
                secret::store_set(&secrets_file, &name, value).map_err(|e| anyhow::anyhow!(e))?;
            if let Some(warning) = warning {
                eprintln!("{warning}");
            }
            eprintln!("set {name} in {}", secrets_file.display());
        }
        SecretsCmd::List { secrets_file } => {
            for name in secret::store_names(&secrets_file).map_err(anyhow::Error::msg)? {
                println!("{name}");
            }
        }
        SecretsCmd::Remove { name, secrets_file } => {
            let removed = secret::store_remove(&secrets_file, &name).map_err(anyhow::Error::msg)?;
            anyhow::ensure!(
                removed,
                "no secret named '{name}' in {}",
                secrets_file.display()
            );
            eprintln!("removed {name} from {}", secrets_file.display());
        }
    }
    Ok(Exit::Completed.into())
}

/// `brokkr driver`: serve a built-in forge-driver/v1 adapter.
pub(crate) fn driver(DriverArgs { kind, args }: DriverArgs) -> Result<ExitCode> {
    let kind = brokkr_protocol::adapters::AdapterKind::parse(&kind).ok_or_else(|| {
        anyhow::anyhow!("unknown driver '{kind}'; known: claude, lanetally, codex, dsh, exec")
    })?;
    let extra = driver_payload(kind, args);
    brokkr_protocol::adapters::serve(kind, extra)?;
    Ok(Exit::Completed.into())
}

/// `brokkr fake-driver`: the scripted driver the machine proof runs.
pub(crate) fn fake_driver(
    FakeDriverArgs {
        script,
        state,
        model,
        effort,
    }: FakeDriverArgs,
) -> Result<ExitCode> {
    brokkr_protocol::fake::run_fake_driver(&script, &state, model.as_deref(), effort.as_deref())?;
    Ok(Exit::Completed.into())
}
