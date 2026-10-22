//! Doctor's capability lines (decision 0065 ruling 4): what each realm
//! grants, and every native capability an installed harness declares,
//! read under each realm's own grants. Moved out of `doctor.rs` by slice
//! two's U9a1; every line reads as it did there.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use brokkr_core::realms::CapabilityGrant;
use brokkr_protocol::native_controls::bounded_line;
use brokkr_runtime::agents::Adapter;
use brokkr_runtime::capabilities::{
    restriction_names, Authority, CapabilityContext, Definitions, Disposition, NativeCapability,
    NativeInventory, Outcome, Requests, Strength, ADAPTER_SEAT, UNMAPPED,
};
use brokkr_runtime::realms::{World, WorldError};
use brokkr_runtime::{Adapters, Availability, Presence};

use super::{safe, Report};
use crate::render::Safe;

/// An adapter-level plan, submitted under one realm's whole authority: the
/// harness, the office of the seat, and what the seat asks for.
type Plan<'a> = dyn Fn(&Adapter, &str, Requests) -> Result<Outcome, String> + 'a;

/// One realm as its capability lines read it: its name, its grants, and
/// how a realm that grants nothing is worded.
struct Realm {
    name: String,
    grants: BTreeMap<String, CapabilityGrant>,
    nothing: &'static str,
}

/// How a realm's grant of one capability was read, each grant validated
/// ALONE (SC5): the realm declares no grant of it, its grant did not
/// validate, or its grant binds one provider's native adapter key.
enum Binding<'a> {
    /// No grant of the capability was read: the realm declares none.
    Unread,
    /// The realm's grant of it did not validate; its failing line is above.
    Failed,
    /// The grant, bound to the native `(provider, key)` it names.
    Native {
        grant: &'a CapabilityGrant,
        provider: String,
        key: String,
    },
}

/// What one realm's native lines are read under: the realm's line prefix,
/// how each of its grants was read, and its whole authority's plan.
struct Under<'a> {
    what: &'a str,
    read: &'a BTreeMap<&'a str, Binding<'a>>,
    plan: &'a Plan<'a>,
}

/// How one grant's office scope reads. The three are different facts and
/// an empty list must never print as "all".
fn scope_words(grant: &CapabilityGrant) -> String {
    match grant.offices.as_deref() {
        None => "all requesting offices".to_string(),
        Some([]) => "no offices".to_string(),
        Some(offices) => format!("offices [{}] only", offices.join(", ")),
    }
}

/// How one declared native control reads, as the adapter data it is: the
/// argv that switches it, the measured default that needs none, the tool
/// lists it contributes to, or the reason it cannot be or has not been
/// measured. Words for a readout, never a claim that the control held.
fn disposition_words(disposition: &Disposition) -> String {
    match disposition {
        Disposition::Argv(argv) => format!("argv [{}]", argv.join(" ")),
        Disposition::Default(reason) => format!("the harness default ({reason})"),
        Disposition::Selection(lists) => format!(
            "tool lists include [{}] allow [{}] deny [{}]",
            lists.include.join(", "),
            lists.allow.join(", "),
            lists.deny.join(", ")
        ),
        Disposition::Unsupported(reason) => format!("cannot be switched ({reason})"),
        Disposition::Unmeasured(reason) => format!("unmeasured ({reason})"),
    }
}

/// Design D8's unknown-authority half: a realms map that could not be read
/// names no realm, so nothing here is keyed to one and nothing is said to
/// be granted, not granted or switched off anywhere — an empty grant set
/// invented for the occasion would be a denial doctor never read. What an
/// installed harness HAS is still adapter data, and is still named: its
/// tools, how ON and OFF are declared, the evidence scope and what stays
/// unmeasured, or the reason its inventory is unmeasured at all.
fn report_native_assessments(report: &mut Report, installed: &[&Adapter]) {
    for adapter in installed {
        let provider = &adapter.provider;
        let natives = match &adapter.native {
            NativeInventory::Known { known, .. } => known,
            NativeInventory::Unmeasured(reason) => {
                report.warn(
                    &format!("capabilities native {provider}"),
                    format!(
                        "native inventory unmeasured: {reason}. No native grant or denial is \
                         claimed"
                    ),
                );
                continue;
            }
        };
        for native in natives.values() {
            report.warn(
                &format!("capabilities native {provider} '{}'", native.capability),
                Safe::new(&format!(
                    "declared by the adapter: tools [{}] · ON: {} · OFF: {} · evidence: {} · \
                     still unmeasured: {} · whether any realm grants it is UNKNOWN, so neither \
                     a grant nor a denial is claimed",
                    native.tools.join(", "),
                    disposition_words(&native.on),
                    disposition_words(&native.off),
                    native.evidence.scope,
                    native.evidence.limitations.join("; ")
                ))
                .as_str()
                .to_string(),
            );
        }
    }
}

/// The scope every adapter-level plan line names (design D8): what was
/// submitted, what was not, and where a seat's own plan is judged.
const ADAPTER_SCOPE: &str = "adapter-level scope: the adapter's own template alone, with no \
                             seat's arguments, model pins, typed tools or hands assessed; a \
                             seat's own plan is judged when its bundle compiles";

/// Decision 0065 ruling 4: per realm, what it grants — by which dialect,
/// to which offices, with which tools and restrictions — and then every
/// native capability an INSTALLED harness declares that the realm has not
/// granted, so the day a realm loses a power it used without permission is
/// loud rather than discovered later.
///
/// A line reports and never refuses, and independent results survive one
/// another (design D8). Each realm is read under ITS OWN grants, never the
/// current directory's, and each GRANT is validated on its own: one that
/// does not validate — a missing definition, a conflicting dialect, an
/// `mcp` grant slice two has not built — is its own failing line carrying
/// the compiler's refusal, the realm's other grants are still shown, and a
/// native power of the failing grant's name reads UNKNOWN under that realm
/// rather than denied, because doctor could not read what would cover it.
/// Definitions that cannot be read are one failing line of their own. A
/// map that could not be read leaves authority UNKNOWN: no empty grant set
/// is invented, and the installed harnesses are still assessed.
///
/// Nothing is run to say any of this: no model request, search, fetch or
/// capability server. "Installed" is the availability `probe_providers`
/// already recorded; a provider whose binary is absent keeps that line and
/// gets no native one, because nothing was observed about it here. A
/// declared control is adapter data — its evidence scope and its open
/// limitations are printed beside it, and a green argv test upgrades
/// neither. Adapter declarations that cannot be read say so here, so a
/// report with no native line is never read as a harness with no native
/// power.
///
/// What a seat on an installed harness is launched with is never judged
/// one capability at a time (operator ruling 4 of 2026-09-23; design D8).
/// Doctor resolves no seat, so it submits the complete adapter-level plan
/// ([`Authority::assess`]) under the realm's whole authority, loaded as a
/// compile loads it: once for a seat that holds nothing, and once for a
/// seat that wants each granted or restricted capability. Each line
/// reports what that plan's resolution and composition admit or refuse,
/// with the refusal's own cause, and names the scope it was assessed at.
///
/// [`Authority::assess`]: brokkr_runtime::capabilities::Authority::assess
pub(super) fn report_capabilities(
    report: &mut Report,
    world: &Result<Option<World>, WorldError>,
    workspace: &Path,
    adapters_root: &Path,
    availability: &Availability,
) {
    let adapters = load_adapters(report, adapters_root);
    let installed: Vec<&Adapter> = adapters
        .iter()
        .flat_map(|adapters| adapters.providers())
        .filter(|adapter| availability.presence(&adapter.provider) == Presence::Available)
        .collect();
    let Some((root, realms)) = mapped_realms(report, world, workspace) else {
        report_native_assessments(report, &installed);
        return;
    };
    if let Err(problem) = Definitions::load(&root) {
        report.missing(
            "capabilities definitions",
            format!(
                "{problem}; no grant can be validated and every compile under this \
                 configuration refuses until it is repaired"
            ),
        );
    }
    for realm in realms {
        report_realm(report, realm, &root, adapters.as_ref(), &installed);
    }
}

/// The adapter declarations, or `None` with a warning when they cannot be
/// read.
fn load_adapters(report: &mut Report, adapters_root: &Path) -> Option<Adapters> {
    match Adapters::load(adapters_root) {
        Ok(adapters) => Some(adapters),
        Err(error) => {
            // A warning on `probe_providers`' terms: a tree with no
            // adapters is a normal state there, and stays one here.
            report.warn(
                "capabilities native",
                format!(
                    "the adapter declarations at {} could not be read ({error}), so no \
                     harness's native capabilities are named below; that is NOT a finding that \
                     an installed harness has none",
                    adapters_root.display()
                ),
            );
            None
        }
    }
}

/// The directory the definitions stand in, and every realm the map names
/// with its grants; `None`, said once, when the map could not be read.
fn mapped_realms(
    report: &mut Report,
    world: &Result<Option<World>, WorldError>,
    workspace: &Path,
) -> Option<(PathBuf, Vec<Realm>)> {
    match world {
        // The definitions stand beside the map, as the engine reads them:
        // the map is a file, so the directory it stands in is always there.
        Ok(Some(world)) => Some((
            workspace
                .join(&world.source)
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_default(),
            world
                .map
                .realms
                .iter()
                .map(|realm| Realm {
                    name: realm.name.clone(),
                    grants: realm.grants.clone(),
                    nothing: "grants nothing",
                })
                .collect(),
        )),
        Ok(None) => Some((
            workspace.to_path_buf(),
            vec![Realm {
                name: UNMAPPED.to_string(),
                grants: BTreeMap::new(),
                nothing: "no realms map, so no capability grants are declared",
            }],
        )),
        Err(_) => {
            report.warn(
                "capabilities",
                "the realms map could not be read, so what each realm grants is UNKNOWN; \
                 nothing is assumed granted and nothing is assumed denied"
                    .into(),
            );
            None
        }
    }
}

/// One realm's lines: each grant's own line, then each installed harness
/// under the realm's whole authority.
fn report_realm(
    report: &mut Report,
    Realm {
        name,
        grants,
        nothing,
    }: Realm,
    root: &Path,
    known: Option<&Adapters>,
    installed: &[&Adapter],
) {
    let what = format!("capabilities {name}");
    if grants.is_empty() {
        report.ok(
            &what,
            format!(
                "{nothing}; every native capability is governed by the no-grant default — \
                 switched off, or the seat is refused"
            ),
        );
    }
    // The realm's whole authority, loaded as a compile under it loads
    // it: a grant that fails refuses every plan in the realm with the
    // compiler's cause, through the one bounded sink (design D6; SC22-1).
    let whole = Authority::load(CapabilityContext {
        realm: name.clone(),
        grants: grants.clone(),
        root: root.to_path_buf(),
    });
    let plan = |adapter: &Adapter, office: &str, asks: Requests| match &whole {
        Ok(authority) => authority.assess(adapter, office, asks),
        Err(problem) => Err(bounded_line(problem)),
    };
    // Each grant is judged ALONE, by the compiler's own validation: one
    // that fails is its own line, read `Failed`, and takes no neighbour.
    let mut read = BTreeMap::new();
    for (capability, grant) in &grants {
        let alone = Authority::load(CapabilityContext {
            realm: name.clone(),
            grants: [(capability.clone(), grant.clone())].into(),
            root: root.to_path_buf(),
        });
        let binding = match report_grant(report, &what, (capability, grant), &alone, known, &plan) {
            Some((provider, key)) => Binding::Native {
                grant,
                provider,
                key,
            },
            None => Binding::Failed,
        };
        read.insert(capability.as_str(), binding);
    }
    let under = Under {
        what: &what,
        read: &read,
        plan: &plan,
    };
    for adapter in installed {
        report_adapter(report, &under, adapter);
    }
}

/// One installed harness under a realm: the plan of a seat that holds none
/// of its native powers, then each power's own line.
fn report_adapter(report: &mut Report, realm: &Under, adapter: &Adapter) {
    let (what, provider) = (realm.what, &adapter.provider);
    let natives = match &adapter.native {
        NativeInventory::Known { known, .. } => known,
        NativeInventory::Unmeasured(reason) => {
            report.warn(
                &format!("{what} native {provider}"),
                format!(
                    "native inventory unmeasured: {reason}. Nothing is granted through \
                     it and no native denial is claimed"
                ),
            );
            return;
        }
    };
    // A seat that holds none of the harness's native powers: every
    // one is switched OFF in ONE plan, and whether that plan is
    // admitted is the answer for all of them together (operator
    // ruling 4 of 2026-09-23). Two OFF controls that each compose
    // alone can refuse together, and a grant, however scoped, leaves
    // such a seat (finding M1).
    let unheld = (realm.plan)(adapter, ADAPTER_SEAT, Default::default());
    let plan_line = format!("{what} plan {provider}");
    match &unheld {
        Ok(_) => report.ok(
            &plan_line,
            format!(
                "{ADAPTER_SCOPE} · a seat on {provider} that holds none of its native \
                 capabilities is admitted, each of them switched off by the composed \
                 command; that is composition, not a live measurement"
            ),
        ),
        Err(cause) => report.warn(
            &plan_line,
            format!(
                "{ADAPTER_SCOPE} · a seat on {provider} that holds none of its native \
                 capabilities is refused, and no denial is claimed: {cause}"
            ),
        ),
    }
    let off = match &unheld {
        Ok(_) => "the adapter-level plan above switches it off",
        Err(_) => "the adapter-level plan above is refused, so no denial is claimed",
    };
    for native in natives {
        report_native(report, realm, adapter, native, (off, unheld.is_ok()));
    }
}

/// One native power of an installed harness under a realm, by how the
/// realm's grant of its name was read; `off` is what the plan of a seat
/// that does not hold it says, and `admitted` whether that plan was.
fn report_native(
    report: &mut Report,
    realm: &Under,
    adapter: &Adapter,
    (key, native): (&String, &NativeCapability),
    (off, admitted): (&str, bool),
) {
    let (what, provider) = (realm.what, &adapter.provider);
    let capability = &native.capability;
    let line = format!("{what} native {provider} '{capability}'");
    let evidence = safe(&format!(
        "evidence: {} · still unmeasured: {}",
        native.evidence.scope,
        native.evidence.limitations.join("; ")
    ));
    let read = realm.read.get(capability.as_str());
    match read.unwrap_or(&Binding::Unread) {
        // A seat that holds it: the plan of a seat the grant
        // reaches that wants it, submitted whole — held with
        // every other power OFF, dropped, or refused.
        Binding::Native {
            grant,
            provider: bound,
            key: bound_key,
        } if bound == provider && bound_key == key => {
            let office = grant_office(grant);
            let (wanting, on) = wanted(realm.plan, adapter, &office, capability);
            let detail = safe(&format!(
                "granted to {} through dialect '{}'; the adapter-level plan of a \
                 seat of office '{office}' that wants it {wanting}; for a seat that \
                 does not hold it, {off} · {evidence}",
                scope_words(grant),
                grant.dialect
            ));
            match on && admitted {
                true => report.ok(&line, detail),
                false => report.warn(&line, detail),
            }
        }
        // The realm DECLARES a grant of this name and doctor
        // could not read it: neither half may be asserted.
        Binding::Failed => report.warn(
            &line,
            format!(
                "UNKNOWN here: this realm's grant of '{capability}' did not \
                 validate (its failing line is above), so neither a grant nor a \
                 denial is claimed, and no seat compiles in this realm until it is \
                 repaired · {evidence}"
            ),
        ),
        Binding::Unread | Binding::Native { .. } => {
            report.warn(&line, format!("NOT granted here: {off} · {evidence}"))
        }
    }
}

/// What the adapter-level plan of a seat of `office` that wants
/// `capability` does with it, and whether that seat holds it ON.
fn wanted(plan: &Plan, adapter: &Adapter, office: &str, capability: &str) -> (String, bool) {
    match plan(adapter, office, wants(capability)) {
        Ok(outcome) if outcome.held.contains_key(capability) => {
            ("is admitted with it ON".to_string(), true)
        }
        Ok(outcome) => (
            format!("drops it ({})", outcome.not_held[capability]),
            false,
        ),
        Err(cause) => (format!("is refused ({cause})"), false),
    }
}

/// The adapter-level asks of a seat that wants `capability` and nothing else.
fn wants(capability: &str) -> Requests {
    [(capability.to_string(), Strength::Wants)].into()
}

/// The office a seat holding a grant is assessed in: the first the grant
/// names, else the hypothesis's own label, which an office list that names
/// none does not reach either.
fn grant_office(grant: &CapabilityGrant) -> String {
    let first = grant.offices.as_deref().and_then(<[String]>::first);
    first.map_or(ADAPTER_SEAT, String::as_str).to_string()
}

/// One grant's line, from its authority loaded ALONE and read through its
/// typed binding (SC5): the native `(provider, key)` it binds, or `None`
/// where the line fails, a binding of another kind in the compiler's words.
pub(super) fn report_grant(
    report: &mut Report,
    what: &str,
    (capability, grant): (&String, &CapabilityGrant),
    alone: &Result<Authority, String>,
    adapters: Option<&Adapters>,
    plan: &Plan,
) -> Option<(String, String)> {
    let line = format!("{what} '{capability}'");
    let bound = alone.as_ref().map_err(String::clone).and_then(|authority| {
        let native = authority.binding(capability);
        native.map_err(|unbound| unbound.to_string())
    });
    let (dialect, provider, key) = match bound {
        Ok(native) => (native.dialect, native.provider, native.adapter_key),
        Err(problem) => {
            report.missing(&line, bounded_line(&problem));
            return None;
        }
    };
    let tools = grant.tools.as_ref().unwrap_or(&dialect.tools);
    let restrictions = match grant.restrictions.is_empty() {
        true => "none".to_string(),
        false => serde_json::Value::Object(grant.restrictions.clone()).to_string(),
    };
    // A declaration is not a claim of usable authority: what the bound provider's adapter says
    // of itself decides whether any seat can hold the capability through this grant.
    let adapter = adapters.and_then(|adapters| adapters.adapter(provider));
    let unusable = match adapter.map(|adapter| (adapter, &adapter.native)) {
        // CQ1 (design D11): no nonempty restriction is expressible in slice one, whatever
        // transport the adapter declares, so a seat that requires the capability is refused and
        // one that wants it drops it. What becomes of the seat that dropped it is its whole
        // plan's to say, never the grant's (finding M1) nor one OFF control's (operator ruling 4).
        Some((adapter, NativeInventory::Known { .. })) if !grant.restrictions.is_empty() => {
            let office = grant_office(grant);
            let wanting = match plan(adapter, &office, wants(capability)) {
                Ok(outcome) => format!("drops it ({})", outcome.not_held[capability]),
                Err(cause) => format!("is refused, and no denial is claimed ({cause})"),
            };
            format!(
                " · restriction '{}' is not usable authority: a seat that requires the capability \
                 is refused, the adapter-level plan of a seat of office '{office}' that wants it \
                 {wanting}, and it never runs unrestricted",
                restriction_names("", &grant.restrictions).join("', '")
            )
        }
        Some((_, NativeInventory::Known { .. })) | None => String::new(),
        Some((_, NativeInventory::Unmeasured(reason))) => format!(
            " · provider '{provider}' declares its native capabilities unmeasured ({reason}): no \
             seat can hold the capability through this grant, and no native denial is claimed"
        ),
    };
    let detail = format!(
        "dialect '{}' ({}, provider '{provider}') · tools [{}] · {} · restrictions \
         {restrictions}{unusable}",
        dialect.name,
        dialect.kind.word(),
        tools.join(", "),
        scope_words(grant),
    );
    report.ok(&line, Safe::new(&detail).as_str().to_string());
    Some((provider.to_string(), key.to_string()))
}
