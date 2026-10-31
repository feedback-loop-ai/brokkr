//! The verbs that prepare the workspace, its libraries and its drivers:
//! `init`, `doctor`, `compile`, `recipes`, `agents`, `muninn`, `secrets`,
//! `driver` and the hidden `fake-driver`.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use brokkr_core::policy::audit::SWEEP_BUDGET;
use brokkr_runtime::bundle::is_engine_owned;
use brokkr_runtime::launch::{self, BundleSource};
use brokkr_runtime::places::{FOLDER, SECRETS};
use brokkr_runtime::realms::World;

use crate::cli_args::{CompileArgs, DoctorArgs, DriverArgs, FakeDriverArgs, InitArgs};
use crate::folder::{DoctorDefaults, Layout};
use crate::recipes::personal::Onto::Project;
use crate::{agents, doctor, init, muninn, recipes};
use crate::{compiled_view, driver_payload, now_rfc3339};
use crate::{world_and_hearths, AgentsCmd, Exit, MuninnCmd, RecipesCmd, SecretsCmd};

/// `brokkr init`: write the `.brokkr/` folder at the repository root
/// `dir`, read for its own manifests so the implement and verify seats
/// are told commands that would actually run there (decision 0083 ruling
/// 5; the operator's ruling 2 of 2026-10-13: one path).
pub(crate) fn init(InitArgs { dir, cli }: InitArgs) -> Result<ExitCode> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    let root = crate::folder::admit::root(&dir)?;
    let home = std::env::var_os("HOME").unwrap_or_default();
    let os = std::env::consts::OS;
    let scaffold = init::init(&root, &path, os, Path::new(&home), cli)?;
    let folder = root.join(FOLDER);
    eprintln!(
        "initialized {} (digest {}), writing:",
        folder.display(),
        scaffold.digest
    );
    for written in &scaffold.written {
        eprintln!("  {}", written.display());
    }
    // Which agent CLI the seats are hired from, and which boundary
    // the realm declares — the two facts read off this machine. Then
    // what SI2 rules on the operator's MCP servers, in the words the
    // scaffold's README uses.
    for note in &scaffold.notes {
        eprintln!("{note}");
    }
    eprintln!("{}", init::AMBIENT_MCP);
    // Decision 0046: the scaffolded seats run under the realm's
    // boundary, and `namespace` — the default — is the one that
    // needs bubblewrap; a realm may declare `harness` instead, and
    // a scaffold that already did asks nothing of bubblewrap.
    if let (true, Err(reason)) = (scaffold.namespace, brokkr_protocol::hands::bwrap_on(&path)) {
        let quoted: Vec<String> = scaffold.hands.iter().map(|s| format!("\"{s}\"")).collect();
        eprintln!(
            "warning: {reason}; the scaffolded seats [{}] declare hands and \
             run under the realm's boundary — `namespace`, \
             the default, needs bubblewrap on PATH, and a realm may declare \
             `harness` instead (decision 0046)",
            quoted.join(", ")
        );
    }
    // Discovery reads the folder from anywhere in the repository once it
    // is committed, and the ship gate closes out only on a clean tree.
    eprintln!("commit the folder: git add {FOLDER} && git commit");
    // A dsh scaffold ends with doctor's own key check (#717), each seat's
    // home modelled from the scaffold's compile, so the remedy is read
    // before a run parks on it.
    let adapters = folder.join(brokkr_runtime::bundle::DEFAULT_ADAPTERS_DIR);
    for line in crate::dsh_key::of_folder(&adapters, &root, Some(&scaffold.bundle)) {
        eprintln!("{}: {}", line.what(), line.detail());
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
    let cwd = std::env::current_dir().unwrap_or_default();
    let report = doctor::doctor(
        bundle.as_deref(),
        db.as_deref(),
        &DoctorDefaults::of(cwd, secrets_file).with_personal(recipes::personal::placed()),
        realms.as_deref(),
    );
    println!("{}", report.render());
    Ok(if report.healthy {
        Exit::Completed.into()
    } else {
        Exit::Failed.into()
    })
}

/// `brokkr compile`: validate a bundle and print its pinned manifest,
/// and decision 0050's sweep of its table on stderr.
pub(crate) fn compile(workspace: &Path, CompileArgs { bundle }: CompileArgs) -> Result<ExitCode> {
    print_compiled(workspace, (&bundle, None))
}

/// What `compile` and `recipes show` both print: the bundle at `leaf`'s
/// directory compiled on the one path a run starts on, in the workspace's
/// realm and across `leaf`'s project shelf, the one `--dir` selected, if
/// any ([`launch::compile_on`]), so the two cannot drift apart (#350,
/// #697). The workspace is the discovered folder's root, where there is
/// one.
fn print_compiled(workspace: &Path, leaf: (&Path, Option<&Path>)) -> Result<ExitCode> {
    // An old layout is refused as every verb refuses it (ruling 4).
    let layout = Layout::discover(workspace)?;
    layout.refuse_old()?;
    let workspace = &layout.workspace();
    let world = World::discover(workspace, None)?;
    let bundle = launch::compile_on(workspace, leaf, world.as_ref(), workspace)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&compiled_view(&bundle, world.as_ref()))?
    );
    // The compiler refused a table past the budget (decision 0050, ruling
    // 4), so the sweep runs; what it can still show is a v1 table's
    // presence finding.
    eprint!(
        "{}",
        bundle.machine.audit_with(SWEEP_BUDGET, is_engine_owned)?
    );
    Ok(Exit::Completed.into())
}

/// `brokkr recipes`: list, add or show a recipe.
pub(crate) fn recipes(workspace: &Path, command: RecipesCmd) -> Result<ExitCode> {
    let layout = Layout::discover(workspace)?;
    layout.refuse_old()?;
    let found = &layout.workspace();
    match command {
        RecipesCmd::List { dir } => {
            let (dir, shelf, on) = layout.listing(dir);
            recipes::personal::list(found, (&dir, on.as_deref()), shelf)?
        }
        RecipesCmd::Add {
            source,
            name,
            path,
            personal,
        } => {
            let (root, shelf) = layout.install_shelf();
            let path = path.as_deref();
            match personal {
                true => {
                    let project = layout.recipes(None);
                    recipes::personal::add(&root, &project, &source, &name, path)?
                }
                false => recipes::add(&root, &shelf, Project, &source, &name, path)?,
            }
        }
        RecipesCmd::Show { name, dir } => {
            let (project, shelf) = layout.selecting(dir);
            let recipe = BundleSource::Recipe {
                recipes_dir: recipes::personal::holding(Some(&name), project)?,
                name,
            };
            return print_compiled(workspace, (&recipe.resolve()?, shelf.as_deref()));
        }
    }
    Ok(Exit::Completed.into())
}

/// `brokkr agents`: list the agent library or show one definition.
pub(crate) fn agents(workspace: &Path, command: AgentsCmd) -> Result<ExitCode> {
    // Semantic lint reads the OPERATOR's capability definitions
    // (decision 0065; design D2): beside the active map, else in
    // the workspace — wherever `--agents-dir` points the library. An old
    // layout is refused as every verb refuses it (decision 0083 ruling 4).
    let layout = Layout::discover(workspace)?;
    layout.refuse_old()?;
    let workspace = &layout.workspace();
    let world = World::discover(workspace, None)?;
    let operator_root = launch::capability_context(workspace, world.as_ref(), None, workspace).root;
    match command {
        AgentsCmd::List { agents_dir } => agents::list(&layout.agents(agents_dir), &operator_root)?,
        AgentsCmd::Show {
            name,
            agents_dir,
            adapters_dir,
        } => {
            let (agents, adapters) = (layout.agents(agents_dir), layout.adapters(adapters_dir));
            agents::show(&name, &agents, &adapters, &operator_root)?
        }
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
            let escape = db.is_some();
            let (world, hearths) = world_and_hearths(workspace, realms, db)?;
            let layout = Layout::discover(workspace)?;
            muninn::run(
                &hearths,
                world.as_ref(),
                &layout.agents(agents_dir),
                &layout.adapters(adapters_dir),
                &record_of(&layout, escape, record)?,
                &now_rfc3339(),
            )
        }
        MuninnCmd::List { record, json } => {
            let layout = Layout::discover(workspace)?;
            layout.refuse_old()?;
            muninn::list(&record_of(&layout, false, record)?, json)?;
            Ok(Exit::Completed.into())
        }
    }
}

/// Muninn's record: the flag, else its place in the run-local state.
fn record_of(layout: &Layout, escape: bool, flag: Option<PathBuf>) -> Result<PathBuf> {
    let place = || layout.state_file(escape, muninn::RECORD);
    Ok(flag.map_or_else(place, Ok)?)
}

/// `brokkr secrets`: set, list or remove a name in the secrets store,
/// the flag's, else the run-local state's (decision 0083).
pub(crate) fn secrets(workspace: &Path, command: SecretsCmd) -> Result<ExitCode> {
    use brokkr_protocol::secret;
    let layout = Layout::discover(workspace)?;
    layout.refuse_old()?;
    let store = |flag: Option<PathBuf>| -> Result<PathBuf> {
        Ok(flag.map_or_else(|| layout.state_file(false, SECRETS), Ok)?)
    };
    match command {
        SecretsCmd::Set { name, secrets_file } => {
            let secrets_file = store(secrets_file)?;
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
            for name in secret::store_names(&store(secrets_file)?).map_err(anyhow::Error::msg)? {
                println!("{name}");
            }
        }
        SecretsCmd::Remove { name, secrets_file } => {
            let secrets_file = store(secrets_file)?;
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
    brokkr_protocol::adapters::serve(kind, extra, confine)?;
    Ok(Exit::Completed.into())
}

/// What a driver sets on itself before it spawns a harness the engine brokers
/// MCP for (U9b1b's ruling F1): `no_new_privs`, which that harness and its
/// broker inherit, as MB3's observer requires of a managed writer. It runs on
/// this process's main thread, the one that spawns the harness and the one
/// `/proc/<pid>/status` reads.
#[cfg(target_os = "linux")]
fn confine() -> std::io::Result<()> {
    rustix::thread::set_no_new_privs(true).map_err(std::io::Error::from)
}

/// No server box stands off Linux, where the MCP fence stands, so a brokered
/// launch never reaches here; one that did refuses.
#[cfg(not(target_os = "linux"))]
fn confine() -> std::io::Result<()> {
    Err(std::io::ErrorKind::Unsupported.into())
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
