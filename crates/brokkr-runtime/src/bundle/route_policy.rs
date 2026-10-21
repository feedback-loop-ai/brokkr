//! The checks of [`enforce_route_policy`](super::enforce_route_policy)
//! (#288, slice H): decision 0021's two prohibitions on one link of a
//! site's route, in the order the link faces them — the gate tier, the
//! judges list and the egress bar. `enforce_route_policy` reads as their
//! sequence over the site's links.

use serde_json::Value;

use super::{destination, unreadable_destination, CompileError, ModelPin, SeatClass};
use crate::agents::{resolve_route, Adapter, EgressClass, TrustTier};

/// One link of a site's route as the prohibitions read it: the site, the
/// link's index, its driver and the adapter that answers for it, the
/// abstract model it hires and its concrete pin, and the driver as a
/// refusal names it.
pub(super) struct RouteLink<'a> {
    pub(super) what: &'a str,
    pub(super) link: usize,
    pub(super) driver: Option<String>,
    pub(super) adapter: Option<&'a Adapter>,
    pub(super) abstract_model: Option<String>,
    pub(super) pin: ModelPin,
    pub(super) named: String,
}

/// Both prohibitions on one link, in their order: a gate link that is no
/// boxed `exec` faces the tier and then the judges, and every link then
/// faces the egress bar.
pub(super) fn judge_link(
    at: &RouteLink<'_>,
    class: SeatClass,
    raw: &Value,
    secrets: &[String],
    minimum: EgressClass,
) -> Result<(), CompileError> {
    if class == SeatClass::Gate && !boxed_exec(at.driver.as_deref(), raw) {
        refuse_untrusted_gate(at)?;
        refuse_unjudged_model(at)?;
    }
    refuse_secret_egress(at, secrets, minimum)
}

/// Decision 0043 ruling 3: a deterministic `exec` command whose hands are
/// boxed has no stochastic axis to distrust, and its blast radius is the
/// box. It may hold a gate.
pub(super) fn boxed_exec(driver: Option<&str>, raw: &Value) -> bool {
    driver == Some("exec") && raw.get("hands").is_some()
}

/// Decision 0021 ruling 2: a gate link's driver holds the trusted tier.
fn refuse_untrusted_gate(at: &RouteLink<'_>) -> Result<(), CompileError> {
    let RouteLink {
        what,
        adapter,
        named,
        ..
    } = at;
    if adapter.map(|a| a.trust_tier) != Some(TrustTier::Trusted) {
        return Err(CompileError::Invalid(format!(
            "seat '{what}' is gate class but seats {named}, which does not \
             hold the trusted tier; a gate seat IS the check, and nobody \
             stands behind the judges (decision 0021 ruling 2 — an \
             undeclared tier is untrusted)"
        )));
    }
    Ok(())
}

/// Decision 0041 ruling 3: a gate link's driver declares its abstract
/// model in `judges`.
fn refuse_unjudged_model(at: &RouteLink<'_>) -> Result<(), CompileError> {
    let RouteLink {
        what,
        link,
        adapter,
        abstract_model,
        pin,
        named,
        ..
    } = at;
    // Decision 0041 ruling 3: trust authorises a provider, while
    // `judges` authorises the particular abstract hire. Both must
    // hold on every fallback link. A missing declaration is the
    // empty set, just like an absent tier is untrusted.
    let admitted = adapter
        .zip(abstract_model.as_deref())
        .is_some_and(|(adapter, model)| adapter.judges.iter().any(|judge| judge == model));
    if !admitted {
        let model = abstract_model.as_deref().unwrap_or(match pin {
            ModelPin::Absent => "<unpinned>",
            _ => "<unmapped>",
        });
        return Err(CompileError::Invalid(format!(
            "seat '{what}' gate link {link} names model '{model}', which {named} does not declare in 'judges' (decision 0041 ruling 3 — an absent declaration is empty)"
        )));
    }
    Ok(())
}

/// Decision 0021 ruling 4 as enacted by 0036 ruling 4: a site that binds
/// secrets reaches no destination below the bundle's egress bar.
fn refuse_secret_egress(
    at: &RouteLink<'_>,
    secrets: &[String],
    minimum: EgressClass,
) -> Result<(), CompileError> {
    let RouteLink {
        what,
        adapter,
        pin,
        named,
        ..
    } = at;
    // Where this site's material actually goes. Two kinds of
    // not-knowing land on the floor for the one ruling-1 reason —
    // an absent declaration is uncontracted — but a refusal has to
    // tell them apart, so each names itself:
    //
    // - a driver NO adapter declares: the operator has said nothing
    //   about this binary's endpoint at all;
    // - a model pin this compiler cannot READ as one concrete id.
    //   Ruling 2 gives an unprefixed id the adapter's own class
    //   because an unprefixed id genuinely arrives at the
    //   destination that class is the operator's word about. A site
    //   that writes a pin has declined that default for somewhere
    //   the machine cannot name, so the adapter's word no longer
    //   covers it — reading the adapter's class here would clear an
    //   unnameable route on the strength of a ruling about a
    //   different one, the same fail-open ruling 2's asymmetry
    //   exists to prevent. `enforce_model_pins` refuses this shape
    //   first for the four model-bearing built-ins, all of which
    //   take `--model`; an adapter the operator ADDS is not on that
    //   list, and this refusal is what stands between it and its
    //   adapter's clearance. It can stand there because the pin
    //   above is read on that adapter's OWN declared `model_flag`
    //   AND on `--model` (decision 0040 ruling 1): an adapter
    //   taking `-m` has no second door to walk an unruled route
    //   through, and no third one on the flag its real CLI also
    //   honours. Nor a fourth by spelling — a word carrying either
    //   flag in a form no reading covers arrives here as
    //   `Unreadable` and is refused, rather than as the silence of
    //   an argv that never named a destination at all.
    let (reached, egress) = match (adapter, &pin) {
        (Some(adapter), ModelPin::Concrete(model)) => destination(resolve_route(adapter, model)),
        // An argv naming no model leaves the binary on whatever its
        // profile resolves, which carries no prefix — so it is
        // literally the unprefixed case, and `resolve_route` stays
        // the one place ruling 2's rule is written.
        (Some(adapter), ModelPin::Absent) => destination(resolve_route(adapter, "")),
        // Named on the flags the read actually used, and only
        // those (decision 0040 ruling 3): no constant stands in for
        // a flag this read did not touch.
        (Some(_), ModelPin::Unreadable(flags)) => {
            (unreadable_destination(flags), EgressClass::Uncontracted)
        }
        (None, _) => destination((None, EgressClass::Uncontracted)),
    };
    if !secrets.is_empty() && egress < minimum {
        return Err(CompileError::Invalid(format!(
            "seat '{what}' declares secret bindings {secrets:?} but seats \
             {named} {reached}, whose egress class is {}; this bundle binds \
             no secret below {} (decision 0021 ruling 4 as enacted by 0036 \
             ruling 4 — an undeclared class is uncontracted, and \
             'egress_minimum' is where the operator rules the bar)",
            egress.name(),
            minimum.name(),
        )));
    }
    Ok(())
}
