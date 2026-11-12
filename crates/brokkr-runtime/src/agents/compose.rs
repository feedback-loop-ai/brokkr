//! Composition of one candidate (#288, slice M): the adapter's driver
//! template, the model pin, the route's declared overlay, the effort half
//! of the hire and the application of the agent's tools, each refusing as
//! it did inline.

use std::path::Path;

use brokkr_protocol::native_controls::{Application, Origin, Segment};

use super::{
    capability_gap, declared_permissions, declared_template, driver_template, lower_allow,
    native_alias, route_is_effortless, Adapter, Adapters, Agent, Availability, BoundaryFragments,
    ChainEntry, Composition, DeclaredDialect, DeclaredRouteOverlay, HarnessHands, Intent, Lowering,
    Presence, ResolveError, ServingInputs,
};

/// Compose one candidate, or refuse. A lookup and a join: there is no
/// template language, so there is no substitution function whose branches
/// could drift from the data. Each contribution is a segment labelled by
/// who supplied it at the point it is made. `workdir` is the run's
/// working directory, used only to spell a declared route overlay's
/// `--patch` value relative to it.
pub(super) fn compose(
    agent: &Agent,
    adapter: &Adapter,
    model: &str,
    concrete: &str,
    boundary: brokkr_core::realms::Boundary,
    declared: Option<&DeclaredRouteOverlay>,
    workdir: &Path,
) -> Result<Composition, ResolveError> {
    let intent = Intent::of(agent);
    let mut segments = vec![driver_template(adapter)];
    // The serving inputs are filled as each contribution is chosen
    // (rebuild unit 14a1), never read back from the segments.
    let mut serving = ServingInputs {
        dialect: DeclaredDialect {
            permissions: declared_permissions(adapter),
            ..DeclaredDialect::default()
        },
        pins: Vec::new(),
        spec: agent.hands.clone(),
    };
    // A provider that serves the model but cannot be TOLD which model is
    // the silent-substitution case in its purest form: it would run its
    // own default and the run would claim the pinned one.
    let flag = adapter.model_flag.as_ref().ok_or_else(|| {
        capability_gap(
            agent,
            adapter,
            model,
            "the provider declares model_flag unsupported, so the chosen model \
             could not be pinned and the provider's own default would run"
                .to_string(),
        )
    })?;
    serving.pins = vec![flag.clone(), concrete.to_string()];
    segments.push(Segment::new(Origin::Template, &serving.pins));

    // Decision 0056's 2026-09-14 note as amended by #1052 ask 2: the
    // route overlay its adapter's route declares is composed here, once,
    // after the model pin. A composition that also authored a `--patch`
    // carries two overlays, which is refused rather than ordered.
    if let Some(declared) = declared {
        if let Some(authored) = authored_patch(&segments) {
            return Err(ResolveError::RouteOverlayConflict {
                agent: agent.name.clone(),
                declared: declared.file.clone(),
                authored,
            });
        }
        let value = declared.patch_value(workdir);
        segments.push(Segment::new(
            Origin::Template,
            &["--patch".to_string(), value],
        ));
    }

    let effort = pin_effort(agent, adapter, model, concrete, &mut segments, &mut serving)?;
    let application =
        tool_application(agent, adapter, model, boundary, &mut segments, &mut serving)?;

    // An agent names no MCP server (decision 0065 ruling 3).
    Ok(Composition {
        segments,
        effort,
        mcp: crate::bundle::McpIntent::composed(intent.hands, boundary),
        intent,
        application,
        template: declared_template(&adapter.driver),
        serving: Box::new(serving),
    })
}

/// An authored `--patch` in the composition so far, in any spelling the
/// engine reads as one, named for the refusal that two overlays earn. The
/// value travels with the flag when it stands in the next token, so the
/// refusal names both overlays and not one flag twice.
fn authored_patch(segments: &[Segment]) -> Option<String> {
    for segment in segments {
        for (at, token) in segment.argv.iter().enumerate() {
            if !token.starts_with("--patch") {
                continue;
            }
            if let Some(value) = segment.argv.get(at + 1).filter(|_| token == "--patch") {
                return Some(format!("{token} {value}"));
            }
            return Some(token.clone());
        }
    }
    None
}

/// The other half of the hire (decision 0035 ruling 5). A model pin
/// without an effort pin is half a hire, and the half it withholds is
/// the half that moves the bill — so an effort-bearing provider that
/// this candidate names no effort for is refused here, exactly as a
/// provider that cannot be told its model is.
pub(super) fn pin_effort(
    agent: &Agent,
    adapter: &Adapter,
    model: &str,
    concrete: &str,
    segments: &mut Vec<Segment>,
    serving: &mut ServingInputs,
) -> Result<Option<String>, ResolveError> {
    let effort = match (&adapter.effort_flag, agent.efforts.get(model)) {
        (None, None) => None,
        (None, Some(effort)) => {
            return Err(capability_gap(
                agent,
                adapter,
                model,
                format!(
                    "the provider declares effort_flag unsupported, so the agent's \
                     effort '{effort}' could not be pinned and the provider's own \
                     default would run"
                ),
            ))
        }
        (Some(_), None) => {
            // Decision 0035 addendum 2026-09-11: a candidate resolving to
            // an effortless route needs no effort entry — the route, not
            // the seat, decides. Every other effort-bearing provider
            // still refuses here.
            if route_is_effortless(adapter, concrete) {
                None
            } else {
                return Err(capability_gap(
                    agent,
                    adapter,
                    model,
                    format!(
                        "the provider takes an effort and this candidate pins none; add \
                         \"efforts\": {{\"{model}\": \"<one of: {}>\"}} to the agent \
                         (decision 0035 ruling 5)",
                        adapter.efforts.join(", ")
                    ),
                ));
            }
        }
        (Some(effort_flag), Some(effort)) => {
            if !adapter.efforts.iter().any(|known| known == effort) {
                return Err(capability_gap(
                    agent,
                    adapter,
                    model,
                    format!(
                        "the provider declares no effort '{effort}'; its vocabulary \
                         is {}",
                        adapter.efforts.join(", ")
                    ),
                ));
            }
            let pinned = [effort_flag.clone(), effort.clone()];
            segments.push(Segment::new(Origin::Template, &pinned));
            serving.pins.extend(pinned);
            Some(effort.clone())
        }
    };
    Ok(effort)
}

/// How the agent's tools apply to this candidate: dormant beside hands,
/// whose fragment a boxed seat must be able to take and whose `harness`
/// fragments an unboxed one carries; lowered onto the adapter's direct
/// flag where the agent lists them; unrestricted where it lists none.
pub(super) fn tool_application(
    agent: &Agent,
    adapter: &Adapter,
    model: &str,
    boundary: brokkr_core::realms::Boundary,
    segments: &mut Vec<Segment>,
    serving: &mut ServingInputs,
) -> Result<Application, ResolveError> {
    let gap = |cause: String| capability_gap(agent, adapter, model, cause);

    let application = if agent.hands.is_some() {
        // The list is dormant beside hands (decision 0043 ruling 2): no
        // direct mapping is required for it and no direct flag is added.
        // Only an entry the adapter DOES map onto a native tool is refused.
        if let (Some(allow), Some(permissions)) = (&agent.allow, &adapter.tool_permissions) {
            for tool in allow {
                if let Some(name) = permissions.names.get(tool) {
                    native_alias(adapter, tool, name).map_err(gap)?;
                }
            }
        }
        if boundary.is_boxed() {
            // Decision 0043 ruling 2: the tool list is not consulted; what
            // the provider must be able to say is how its harness is handed
            // the boxed tool (a Codex seat keeps its read-only native shell).
            let fragment = adapter.hands.as_ref().ok_or_else(|| {
                let declared = match &adapter.hands_gap {
                    Some(reason) => format!("the provider declares hands unsupported ({reason})"),
                    None => "the provider declares hands unsupported".to_string(),
                };
                capability_gap(
                    agent,
                    adapter,
                    model,
                    format!(
                        "{declared}, so the agent's hands cannot be put in the box and \
                     the agent would run with the harness's own tools"
                    ),
                )
            })?;
            segments.push(Segment::new(Origin::Hands, fragment));
            serving.dialect.hands = fragment.clone();
        } else if boundary == brokkr_core::realms::Boundary::Harness {
            // Decision 0046 ruling 4: unboxed under `harness`, the engine
            // appends the fragment the seat's class selects behind the
            // command, as the adapter declares it.
            serving.dialect.boundary = BoundaryFragments {
                gate: adapter.harness.gate.clone().unwrap_or_default(),
                work: adapter.harness.work.clone().unwrap_or_default(),
            };
        }
        // Hands replace the harness's tools: a declared list is kept as
        // intent, and its concrete mapping is inapplicable, not absent.
        Application::Dormant
    } else if let Some(allow) = &agent.allow {
        let lowered = lower_allow(adapter, allow, "agent").map_err(gap)?;
        segments.push(lowered.segment);
        Application::Direct(lowered.limits)
    } else {
        Application::Unrestricted
    };
    Ok(application)
}

pub(super) fn entry_for(
    agent: &Agent,
    adapters: &Adapters,
    availability: &Availability,
    model: &str,
    boundary: brokkr_core::realms::Boundary,
    workdir: &Path,
) -> ChainEntry {
    let Some((adapter, concrete)) = adapters.serving(model) else {
        return ChainEntry {
            model: model.to_string(),
            effort: None,
            provider: None,
            presence: Presence::Unknown,
            argv: Vec::new(),
            hands_fragment: Vec::new(),
            harness: HarnessHands::default(),
            route_overlay: None,
            gap: None,
            notices: Vec::new(),
            lowering: Lowering::Unavailable,
        };
    };
    let presence = availability.presence(&adapter.provider);
    let route_overlay = adapters.route_overlay(&adapter.provider, concrete);
    // The flat fields are projections of the composition, never a second
    // derivation beside it; a refused entry keeps only its typed intent.
    let (argv, effort, hands_fragment, gap, lowering) = match compose(
        agent,
        adapter,
        model,
        concrete,
        boundary,
        route_overlay,
        workdir,
    ) {
        Ok(composition) => (
            composition.argv(),
            composition.effort.clone(),
            composition.hands_fragment(),
            None,
            Lowering::Composed(composition),
        ),
        Err(gap) => (
            Vec::new(),
            None,
            Vec::new(),
            Some(gap),
            Lowering::Refused(Intent::of(agent)),
        ),
    };
    ChainEntry {
        model: model.to_string(),
        effort,
        provider: Some(adapter.provider.clone()),
        presence,
        argv,
        hands_fragment,
        harness: adapter.harness.clone(),
        route_overlay: route_overlay.cloned(),
        gap,
        // Filled by the compiler, which knows the realm: a capability the
        // office wants and the realm does not grant (decision 0065).
        notices: Vec::new(),
        lowering,
    }
}

/// The composition of a test that declares no route overlay, so a test
/// asserting an ordinary composition keeps its old call shape rather than
/// spelling the overlay argument it is not about.
#[cfg(test)]
pub(super) fn compose_test(
    agent: &Agent,
    adapter: &Adapter,
    model: &str,
    concrete: &str,
    boundary: brokkr_core::realms::Boundary,
) -> Result<Composition, ResolveError> {
    compose(
        agent,
        adapter,
        model,
        concrete,
        boundary,
        None,
        Path::new(""),
    )
}

#[cfg(test)]
mod tests;
