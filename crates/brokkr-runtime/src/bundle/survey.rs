//! A survey of a bundle's seats (decision 0065 slice two, U9a2; SD3): each
//! executable site's whole capability plan read through the compile's own
//! seat pass, its candidate and inline planners, their MCP contribution,
//! U1g1's verdicts and the dialect wrapper's validator and relocation
//! included, under an authority read past the realm-wide refusals a compile
//! meets first ([`Authority::survey`]). Facts to report, never a bundle:
//! the pass stops once every site's plan is recorded, and nothing it reads
//! reaches a compile or a launch. A compile of the same bundle still
//! refuses every `mcp` grant until U9b lifts the fence. The seat pass both
//! share ([`seat`]) and how each meets a site's refusal ([`Refusals`]) live
//! here, beside the leaf composition both read first.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use brokkr_protocol::adapters::StrictCause;

use super::assemble::{compile_seats, hold_own_drivers, open_agents, wrapped_verify, Seated};
use super::mcp::Planned;
use super::{compose, parse_egress_minimum, phases, relocate_verify_facts};
use super::{Charters, CompileError, RealmLaw};
use super::{SiteCompiler, SiteKey};
use crate::agents::EgressClass;
use crate::argv::Executable;
use crate::capabilities::{Authority, CapabilityContext, SiteCapabilities, UNMAPPED};
use crate::dialect::Dialect;
use crate::launch::{capability_context, LaunchError};
use crate::places::{personal_shelf_here, Layout, Shelves};
use crate::realms::World;

/// What a compile would judge of a bundle's seats, read past the MCP
/// compile fence and never compiled past it. Every map is keyed as a
/// compiled bundle's sites are, after the dialect wrapper relocates them.
#[derive(Debug)]
pub struct Survey {
    /// The binding minimum every `mcp` dialect's egress is judged against
    /// (MB4): the bundle's, parsed as its compile parses it, or with no
    /// bundle the default a bundle that declares none gets. Kept whatever
    /// the pass meets after it was read.
    pub minimum: EgressClass,
    /// Each executable site's capability plan, one outcome per serving
    /// with its SI2 record.
    pub sites: BTreeMap<String, SiteCapabilities>,
    /// Each site whose capability record refuses, with the compile's own
    /// refusal; the walk went on to its siblings and the seats after it.
    pub refused: BTreeMap<String, CompileError>,
    /// U1g1's verdict on each serving of every site the planner reached,
    /// planned or refused, in serving order: the cause SI2's admission
    /// refuses it with once U9b lifts the fence, or `None`.
    pub strict: BTreeMap<String, Vec<Option<StrictCause>>>,
    /// What stopped the pass where no one site owns it, the compile's own
    /// refusal; what was read before it stands above.
    pub failed: Option<CompileError>,
}

impl Survey {
    /// A survey under `minimum` that has read no site yet.
    fn at(minimum: EgressClass) -> Survey {
        Survey {
            minimum,
            sites: BTreeMap::new(),
            refused: BTreeMap::new(),
            strict: BTreeMap::new(),
            failed: None,
        }
    }
}

/// The survey of the bundle at `dir`, read as [`crate::launch::compile_for`]
/// compiles it from `workspace` for the operated `repo` under `world`: the
/// same realm, dialect, boundary, shelves and capability context, its
/// realm's grants validated and none refused realm-wide. With no bundle,
/// the default binding minimum and no seat. Held to the compile by
/// `a_bundle_compile_admits_is_surveyed_seat_for_seat_as_it_compiles`.
pub fn survey(
    workspace: &Path,
    dir: Option<&Path>,
    world: Option<&World>,
    repo: &Path,
) -> Result<Survey, LaunchError> {
    let Some(dir) = dir else {
        return Ok(Survey::at(crate::capabilities::ABSENT_EGRESS_MINIMUM));
    };
    let realm = world.and_then(|world| world.realm_for(repo));
    let dialect = match world.zip(realm) {
        Some((world, realm)) => world.dialect_for_realm(realm)?,
        None => None,
    };
    let layout = Layout::discover(workspace)?;
    let shelves = Shelves::of(&layout, None, personal_shelf_here().ok());
    let places = layout.places();
    let (_, resolved) = composed(dir, &shelves)?;
    let name = realm.map_or(UNMAPPED, |realm| realm.name.as_str());
    let law = (Some(name), dialect, RealmLaw::of(world, realm));
    let context = capability_context(workspace, world, realm, repo);
    let roots = (places.agents.as_path(), places.adapters.as_path());
    Ok(surveyed(&resolved, roots, law, &context)?)
}

/// The bundle at `dir`, canonicalized and composed across `shelves` into
/// one flat bundle, as every compile and survey reads it first: everything
/// after this compiles a single bundle and never learns that composition
/// happened (decision 0017).
pub(super) fn composed(
    dir: &Path,
    shelves: &Shelves,
) -> Result<(PathBuf, compose::Resolved), CompileError> {
    let dir = dir
        .canonicalize()
        .map_err(|e| CompileError::Invalid(format!("bundle dir {}: {e}", dir.display())))?;
    let resolved = compose::resolve_unsealed(&dir, shelves)?;
    Ok((dir, resolved))
}

/// The compile's own steps up to and through its dialect wrapper, under the
/// surveyed authority, each site's refusal kept as its own. The binding
/// minimum is read first, by the parser the table's own lints call, so a
/// failure after it, the rest of the table's lints and the realm's grants
/// included, is the survey's `failed` and never loses it.
fn surveyed(
    resolved: &compose::Resolved,
    roots: (&Path, &Path),
    (realm_name, dialect, law): (Option<&str>, Option<&Dialect>, RealmLaw),
    capabilities: &CapabilityContext,
) -> Result<Survey, CompileError> {
    let minimum = parse_egress_minimum(&resolved.document)?;
    let mut survey = Survey::at(minimum);
    let table = phases::table(resolved, realm_name, dialect);
    let planned = table.and_then(|table| {
        let authority = Authority::survey(capabilities.clone());
        authority.and_then(|authority| {
            let at = (&authority.with_minimum(minimum), resolved);
            plan(&mut survey, at, roots, (dialect, law), &table)
        })
    });
    survey.failed = planned.err();
    Ok(survey)
}

/// Every site's plan under `authority`, written into `survey` as far as
/// the compile's seat pass, capability pass and dialect wrapper go.
fn plan(
    survey: &mut Survey,
    (authority, resolved): (&Authority, &compose::Resolved),
    roots: (&Path, &Path),
    (dialect, law): (Option<&Dialect>, RealmLaw),
    table: &phases::Table,
) -> Result<(), CompileError> {
    let (engine, charters) = (Executable::current(), Charters::default());
    let at = (authority, resolved, &engine, &charters);
    let mut seated = seat(at, roots, (dialect, law), table)?;
    let refusals = &mut Refusals::Survey(&mut *survey);
    let passed = (seated.settle(&table.machine, roots.1, refusals))
        .and_then(|()| seated.wrap(wrapped_verify(table.uses_dialect, dialect), refusals));
    survey.sites = (seated.sites.into_iter())
        .filter_map(|(label, facts)| Some(label).zip(facts.capabilities))
        .collect();
    passed
}

/// The seat pass a compile and a survey share, under `authority`, which
/// already judges by the bundle's binding minimum: what the seats open,
/// every composed seat compiled and the drivers they name held, ready to
/// settle.
pub(super) fn seat<'s>(
    (authority, resolved, engine, charters): Under<'s>,
    roots: (&'s Path, &'s Path),
    (dialect, law): (Option<&'s Dialect>, RealmLaw),
    table: &phases::Table,
) -> Result<Seated<'s, 's>, CompileError> {
    let RealmLaw {
        boundary,
        provisional_offices,
    } = law;
    let (minimum, uses_dialect) = (authority.minimum(), table.uses_dialect);
    let definitions = &authority.definitions;
    let agents = open_agents(
        roots,
        definitions,
        minimum,
        provisional_offices,
        uses_dialect,
    )?;
    let wrapped_verify = wrapped_verify(uses_dialect, dialect);
    // The one canonical family table (design D10 F1) is the compiler's
    // until the loop ends: every fact is written into an entrant of its
    // map; a wrapper moves entrants.
    let (roots, case_origin) = (&resolved.roots, &resolved.case_origin);
    let at = (boundary, engine);
    let mut compiler = SiteCompiler::new(agents, at, dialect, charters, roots, case_origin);
    let machine = &table.machine;
    let (seats, verify_agent_hands) =
        compile_seats(&mut compiler, machine, resolved, wrapped_verify)?;
    hold_own_drivers(&compiler, machine, resolved, wrapped_verify)?;
    let SiteCompiler { agents, sites, .. } = compiler;
    Ok(Seated {
        authority,
        boundary,
        engine,
        resolved,
        agents,
        sites,
        seats,
        verify_agent_hands,
    })
}

/// What the seat pass stands under: the realm's authority, the
/// composition, the engine executable a `{brokkr}` expands to, and the
/// charters the seats bind.
pub(super) type Under<'s> = (
    &'s Authority,
    &'s compose::Resolved,
    &'s Executable,
    &'s Charters,
);

/// How the capability pass meets a site whose record refuses: a compile
/// refuses the bundle with it, and a survey keeps it as that site's, with
/// the SI2 verdicts its planner read, and walks on, so every site's plan
/// is read.
pub(super) enum Refusals<'r> {
    Compile,
    Survey(&'r mut Survey),
}

impl Refusals<'_> {
    /// `recorded`, the capability record of the executable site `site`, as
    /// this pass meets it.
    pub(super) fn meet(
        &mut self,
        site: &str,
        recorded: Result<(), CompileError>,
    ) -> Result<(), CompileError> {
        match (self, recorded) {
            (Refusals::Compile | Refusals::Survey(_), Ok(())) => Ok(()),
            (Refusals::Compile, Err(refusal)) => Err(refusal),
            (Refusals::Survey(survey), Err(refusal)) => {
                survey.refused.insert(site.to_string(), refusal);
                Ok(())
            }
        }
    }

    /// The plan of `site`: a survey keeps each serving's SI2 verdict
    /// whatever its resolution came to, and both passes go on with its
    /// capabilities or its refusal.
    pub(super) fn planned(
        &mut self,
        site: &str,
        planned: Planned,
    ) -> Result<SiteCapabilities, CompileError> {
        match self {
            Refusals::Compile => {}
            Refusals::Survey(survey) => {
                survey.strict.insert(site.to_string(), planned.strict);
            }
        }
        planned.capabilities
    }

    /// The dialect wrapper's relocation of `moved`, applied to what a
    /// survey keeps per site as it is to the site family.
    pub(super) fn relocate(&mut self, moved: &[(String, String, SiteKey)]) {
        match self {
            Refusals::Compile => {}
            Refusals::Survey(survey) => {
                relocate_verify_facts(&mut survey.refused, moved);
                relocate_verify_facts(&mut survey.strict, moved);
            }
        }
    }
}

#[cfg(test)]
mod tests;
