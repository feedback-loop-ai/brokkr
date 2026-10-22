//! A start's door (#288): the run id [`Engine::start_in_world`] names, and
//! every fence it runs before `create_run`, in the order it runs them.
//! What [`door`] returns is what the start journals; a refusal here
//! journals nothing and spawns no seat.
//!
//! [`Engine::start_in_world`]: super::Engine::start_in_world

use std::path::{Path, PathBuf};

use brokkr_core::realms::Boundary;
use serde_json::{json, Map, Value};
use uuid::Uuid;

use super::{
    moved_capability_input, operated_repo, refuse_moved_charter, refuse_unbuilt, EngineError,
};
use crate::bundle::Bundle;
use crate::realms::World;

/// What a start admitted at its door: the run's id, the repository it
/// operates, the charter bindings it checked, and the manifest it pins.
pub(super) struct Admitted {
    pub(super) run_id: String,
    pub(super) operated: PathBuf,
    pub(super) charters: Value,
    pub(super) manifest: Value,
}

/// The door [`Engine::start_in_world`] runs before `create_run`: the run
/// id, then the unbuilt, boundary, capability and charter fences, then
/// the manifest pin.
///
/// [`Engine::start_in_world`]: super::Engine::start_in_world
pub(super) fn door(
    bundle: &Bundle,
    feature: &str,
    repo: Option<PathBuf>,
    world: Option<&World>,
) -> Result<Admitted, EngineError> {
    let run_id = run_id_for(feature);
    // Decision 0046 ruling 1 at the door, before `create_run` and
    // before any row: the world this run is started with must resolve
    // the word the bundle was compiled under for the operated
    // repository — and no world resolves `namespace`, which is what
    // every run meant before the word existed (design DD3).
    refuse_unbuilt(bundle)?;
    // The operated repository is what `workdir()` will answer: the
    // named one, else the directory the engine stands in.
    let operated = operated_repo(repo);
    boundary_fence(bundle, world, &operated)?;
    capability_fence(bundle, world, &operated)?;
    // And every charter the bundle bound, where its owner stands now
    // (rebuild unit 19): no run is journaled over one that moved. The
    // bindings checked are recorded with the run, so its resume, over a
    // bundle compiled again, is held to them (review return F1).
    let charters = refuse_moved_charter(bundle)?;
    // Pinned for the same operated repository the fence judged, so a
    // run started from a mapped workspace with no `--repo` still
    // names its realm — and resumes under the word it was started
    // with, which `compile_from_manifest` reads off that pin.
    let manifest = match world {
        Some(world) => world.pinned(&bundle.manifest, Some(&operated))?,
        None => bundle.manifest.clone(),
    };
    Ok(Admitted {
        run_id,
        operated,
        charters,
        manifest,
    })
}

/// The feature's slug — lowercased, every run of non-alphanumerics one
/// `-`, at most 32 characters — and 8 characters of a fresh uuid.
fn run_id_for(feature: &str) -> String {
    let slug: String = feature
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let slug = slug.chars().take(32).collect::<String>();
    format!("{slug}-{}", &Uuid::new_v4().to_string()[..8])
}

/// The boundary the world resolves for the operated repository, else
/// `namespace`, must be the one the bundle was compiled under.
fn boundary_fence(
    bundle: &Bundle,
    world: Option<&World>,
    operated: &Path,
) -> Result<(), EngineError> {
    let resolved = match world {
        Some(world) => world.boundary_for(operated),
        None => Boundary::Namespace,
    };
    if resolved != bundle.boundary {
        return Err(EngineError::BoundaryMismatch {
            compiled: bundle.boundary,
            world: resolved,
        });
    }
    Ok(())
}

/// The realm and grants the bundle was compiled under must be the ones
/// the world declares for the operated repository today, and the bytes
/// they were judged against must not have moved.
fn capability_fence(
    bundle: &Bundle,
    world: Option<&World>,
    operated: &Path,
) -> Result<(), EngineError> {
    // Decision 0065 ruling 3, fenced the same way and for the same
    // reason: the grants the operated realm declares TODAY are what
    // this run may hold, and a bundle compiled under any other grant
    // context — a neighbouring realm's, an edited map's, none at all
    // — is refused before `create_run`, so nothing is journaled and no
    // seat spawns.
    let operated_realm = world.and_then(|world| world.realm_for(operated));
    let world_realm = operated_realm.map_or(crate::capabilities::UNMAPPED, |realm| &realm.name);
    let world_grants: Map<String, Value> = operated_realm
        .into_iter()
        .flat_map(|realm| &realm.grants)
        .map(|(capability, grant)| (capability.clone(), grant.value()))
        .collect();
    // What is compared is the realm and its GRANTS, which is the
    // authority: a bundle that records none was compiled under none,
    // and a neighbouring realm granting the very same map is still a
    // different office-holder's permission, not this one's.
    let compiled = &bundle.manifest["capabilities"];
    let compiled_grants = compiled.get("grants").cloned().unwrap_or_else(|| json!({}));
    let compiled_realm = compiled["realm"]
        .as_str()
        .unwrap_or(crate::capabilities::UNMAPPED);
    if compiled_grants != json!(world_grants) || compiled_realm != world_realm {
        return Err(EngineError::CapabilityMismatch {
            compiled_realm: compiled_realm.to_string(),
            compiled: compiled_grants.to_string(),
            world_realm: world_realm.to_string(),
            world: Value::Object(world_grants).to_string(),
        });
    }
    // And the bytes those grants were judged against, read where the
    // compile read them: beside the map, else under the operated
    // repository (design D2).
    let operator_root = match world {
        Some(world) => world.source.parent().map(PathBuf::from).unwrap_or_default(),
        None => operated.to_path_buf(),
    };
    if let Some((input, problem)) = moved_capability_input(compiled, &operator_root) {
        return Err(EngineError::CapabilityInputMoved { input, problem });
    }
    Ok(())
}
