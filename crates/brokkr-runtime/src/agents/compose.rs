//! The steps of [`compose`](super::compose) (#288, slice M): the effort
//! half of a candidate's hire and the application of the agent's tools,
//! each refusing as it did inline. `compose` reads as the model pin, then
//! these two, then the composition.

use brokkr_protocol::native_controls::{Application, Origin, Segment};

use super::{
    capability_gap, lower_allow, native_alias, route_is_effortless, Adapter, Agent,
    BoundaryFragments, ResolveError, ServingInputs,
};

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
