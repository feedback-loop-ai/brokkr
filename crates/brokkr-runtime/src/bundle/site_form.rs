//! A site's form, read once (#349). A seat, a select case, a panel member
//! and a sequence step each write one body form among those their own
//! [`SiteForm`] admits, and [`classify`] reads which, after the refusals
//! every site shares, in one order at all four: boundary, crossing,
//! unknown keys, confine, driver keys, amendments, exactly one admitted
//! form, then tools beside a container. Each site dispatches on its own
//! result enum, so no site holds an arm for a form it cannot write.

use serde_json::Value;

use super::{bounded_site, CompileError};

/// Where an agent reference sits. A seat OWNS its 0006 bounds and its
/// 0007 declaration; a panel member or sequence step has neither — the
/// seat above it does. So an agent carrying `limits` or `inputs` cannot
/// be referenced from a member or a step: silently discarding them would
/// make `brokkr agents show` a lie for that site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SiteRole {
    Seat,
    Member,
}

/// A body form a site may write, named by the word its refusal says.
#[derive(Clone, Copy)]
enum Shape {
    Inline,
    Agent,
    Panel,
    Sequence,
    Select,
    Dialect,
}

impl Shape {
    fn word(self) -> &'static str {
        match self {
            Shape::Inline => "role+driver",
            Shape::Agent => "agent",
            Shape::Panel => "panel",
            Shape::Sequence => "sequence",
            Shape::Select => "select",
            Shape::Dialect => "dialect",
        }
    }

    /// Whether `raw` writes this form: an inline site writes `role` or
    /// `driver` and no `agent`; every other form is its own key.
    fn written_in(self, raw: &Value) -> bool {
        match self {
            Shape::Inline => {
                raw.get("agent").is_none()
                    && (raw.get("role").is_some() || raw.get("driver").is_some())
            }
            Shape::Agent | Shape::Panel | Shape::Sequence | Shape::Select | Shape::Dialect => {
                raw.get(self.word()).is_some()
            }
        }
    }

    /// A form that holds other sites and executes nothing of its own. The
    /// match names every form, so a new one cannot skip the tools refusal.
    fn contains(self) -> bool {
        match self {
            Shape::Panel | Shape::Sequence | Shape::Select => true,
            Shape::Inline | Shape::Agent | Shape::Dialect => false,
        }
    }
}

/// The forms a seat may write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SeatShape {
    Inline,
    Agent,
    Panel,
    Sequence,
    Select,
}

/// The forms a select case or default may write: a seat's, but no select.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CaseShape {
    Inline,
    Agent,
    Panel,
    Sequence,
}

/// The forms a sequence step may write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StepShape {
    Inline,
    Agent,
    Panel,
    Dialect,
}

/// The forms a panel member may write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MemberShape {
    Inline,
    Agent,
}

/// One site's form: its role, the noun its "exactly one of" refusal
/// names it by, its closed key vocabulary, the forms it admits in the
/// order that refusal lists them, and the form a site writing none of
/// them reads as, if any.
pub(super) struct SiteForm<S: 'static> {
    pub(super) role: SiteRole,
    noun: &'static str,
    keys: &'static [&'static str],
    shapes: &'static [(Shape, S)],
    bare: Option<S>,
}

/// A seat: a site writing no form is inline, and its own `role` and
/// `driver` refusals speak.
pub(super) const SEAT: SiteForm<SeatShape> = SiteForm {
    role: SiteRole::Seat,
    noun: "seat",
    keys: SEAT_KEYS,
    shapes: &[
        (Shape::Inline, SeatShape::Inline),
        (Shape::Agent, SeatShape::Agent),
        (Shape::Panel, SeatShape::Panel),
        (Shape::Sequence, SeatShape::Sequence),
        (Shape::Select, SeatShape::Select),
    ],
    bare: Some(SeatShape::Inline),
};

/// A select case or default, which must write a form.
pub(super) const CASE: SiteForm<CaseShape> = SiteForm {
    role: SiteRole::Seat,
    noun: "seat",
    keys: BODY_KEYS,
    shapes: &[
        (Shape::Inline, CaseShape::Inline),
        (Shape::Agent, CaseShape::Agent),
        (Shape::Panel, CaseShape::Panel),
        (Shape::Sequence, CaseShape::Sequence),
    ],
    bare: None,
};

/// A panel member.
pub(super) const MEMBER: SiteForm<MemberShape> = SiteForm {
    role: SiteRole::Member,
    noun: "seat",
    keys: MEMBER_KEYS,
    shapes: &[
        (Shape::Inline, MemberShape::Inline),
        (Shape::Agent, MemberShape::Agent),
    ],
    bare: Some(MemberShape::Inline),
};

/// A sequence step, which sits where a member does.
pub(super) const STEP: SiteForm<StepShape> = SiteForm {
    role: SiteRole::Member,
    noun: "sequence step",
    keys: STEP_KEYS,
    shapes: &[
        (Shape::Inline, StepShape::Inline),
        (Shape::Agent, StepShape::Agent),
        (Shape::Panel, StepShape::Panel),
        (Shape::Dialect, StepShape::Dialect),
    ],
    bare: Some(StepShape::Inline),
};

/// The form `raw` writes at the site `what`, after the refusals every
/// site shares, in the one order this module states.
pub(super) fn classify<S: Copy>(
    what: &str,
    raw: &Value,
    form: &SiteForm<S>,
) -> Result<S, CompileError> {
    admit(what, raw, form.keys)?;
    let shapes: Vec<Shape> = form.shapes.iter().map(|(shape, _)| *shape).collect();
    written_form(what, raw, form.noun, &shapes)?
        .map(|index| form.shapes[index].1)
        .or(form.bare)
        .ok_or_else(|| exactly_one(what, form.noun, &shapes))
}

/// The refusals every site shares, before its form is read.
fn admit(what: &str, raw: &Value, keys: &[&str]) -> Result<(), CompileError> {
    refuse_boundary_key(what, raw)?;
    refuse_crossing_keys(what, raw)?;
    refuse_unknown_keys(what, raw, keys)?;
    refuse_confine(what, raw)?;
    refuse_driver_keys(what, raw)?;
    match raw.get("agent") {
        None => Ok(()),
        Some(_) => refuse_amendments(what, raw),
    }
}

/// The index of the one form in `shapes` that `raw` writes, or `None`
/// when it writes none. Two written are refused, and so is `tools` beside
/// a container.
fn written_form(
    what: &str,
    raw: &Value,
    noun: &str,
    shapes: &[Shape],
) -> Result<Option<usize>, CompileError> {
    let written: Vec<usize> = (0..shapes.len())
        .filter(|index| shapes[*index].written_in(raw))
        .collect();
    match written.as_slice() {
        [] => Ok(None),
        [index] => {
            if shapes[*index].contains() {
                refuse_tools_on_container(what, raw)?;
            }
            Ok(Some(*index))
        }
        _ => Err(exactly_one(what, noun, shapes)),
    }
}

/// The refusal of a site that writes two forms, or none where it must
/// write one: its forms, listed with ", " and ", or " before the last.
fn exactly_one(what: &str, noun: &str, shapes: &[Shape]) -> CompileError {
    let words: Vec<&str> = shapes.iter().map(|shape| shape.word()).collect();
    let last = words.len() - 1;
    CompileError::Invalid(format!(
        "{noun} '{what}' must be exactly one of {}, or {}",
        words[..last].join(", "),
        words[last]
    ))
}

/// An agent reference is TOTAL: a seat that could amend the agent it
/// names would make `agent: implementer` stop being a complete statement
/// about what ran — inlining with extra steps, and drift with a name on
/// it. `results`, `secrets` and `class` stay legal beside it, because
/// they are bindings the SEAT provides rather than statements about what
/// the agent is, and `brokkr agents show` never claims to show them.
/// `limits` is likewise a strategy bound on the invocation, not part of
/// the office. `class` in particular is the seat's authority, never the
/// agent's: one charter may sit in a work seat here and a gate seat
/// there, which is why decision 0021 ruling 1 puts the division in bundle
/// data. `driver.confine` was the one `driver` key legal here until
/// decision 0046 ruling 5 retired it; [`refuse_confine`] now refuses it
/// by name before this lint reads the driver at all.
pub(super) fn refuse_amendments(what: &str, raw: &Value) -> Result<(), CompileError> {
    let refuse = |key: &str| {
        Err(CompileError::Invalid(format!(
            "seat '{what}' combines 'agent' with '{key}'; an agent reference is \
             total — '{key}' states what the agent IS, and a seat that could \
             amend it would make `brokkr agents show` a lie for that seat"
        )))
    };
    for key in ["role", "inputs", "hands"] {
        if raw.get(key).is_some() {
            return refuse(key);
        }
    }
    if let Some(driver) = raw.get("driver") {
        let object = driver.as_object().ok_or_else(|| {
            CompileError::Invalid(format!("seat '{what}' driver must be an object"))
        })?;
        if let Some(key) = object.keys().next() {
            return refuse(&format!("driver.{key}"));
        }
    }
    Ok(())
}

/// Decision 0046 ruling 5: decision 0008's `driver.confine` retired into
/// the `container` boundary the realm declares, and until slice (iii)
/// measures that boundary the field is refused wherever it is written —
/// in a bundle and beside `agent:` alike — so a bundle that still carries
/// it fails loudly rather than running a wrapper nobody exercised. No
/// shipped bundle declares it, so nothing shipped moves for this reason.
pub(super) fn refuse_confine(what: &str, raw: &Value) -> Result<(), CompileError> {
    match raw.pointer("/driver/confine") {
        None => Ok(()),
        Some(_) => Err(CompileError::Invalid(format!(
            "seat '{what}' declares driver.confine; decision 0008's container \
             confinement retired into the `container` boundary a realm declares \
             (realms.json, forge.realms/v4), and the field is refused until that \
             boundary is measured in decision 0046's slice (iii) (decision 0046 \
             ruling 5)"
        ))),
    }
}

/// The keys a site's `driver` object may write. Closed, like the site's
/// own vocabulary ([`refuse_unknown_keys`]): the compiler reads only
/// `command` there (and refuses `confine` by name first, in
/// [`refuse_confine`]), so before rebuild unit 5e a `tools` object placed
/// under `driver` compiled, delivered nothing and ran the seat at its
/// harness default.
const DRIVER_KEYS: &[&str] = &["command"];

/// Decision 0004's closed input semantics, applied to the `driver`
/// object (decision 0065 slice one, rebuild unit 5e): an unknown key is
/// refused where it is written, never ignored. The reason names the key
/// and the object, and never the value; a key that is not a short name is
/// described rather than echoed, so the reason stays bounded. A
/// capability key gets the place it belongs.
fn refuse_driver_keys(what: &str, raw: &Value) -> Result<(), CompileError> {
    let Some(driver) = raw.get("driver").and_then(Value::as_object) else {
        return Ok(());
    };
    let Some(key) = driver
        .keys()
        .find(|key| !DRIVER_KEYS.contains(&key.as_str()))
    else {
        return Ok(());
    };
    let named = if key.len() <= 64
        && key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_-.".contains(&byte))
    {
        format!("'{key}'")
    } else {
        "one that is not a short name and is not echoed".to_string()
    };
    let place = match key.as_str() {
        "tools" | "hands" | "capabilities" => {
            format!(" '{key}' is a site declaration, written on the seat beside its driver.")
        }
        "sandbox" => {
            " 'sandbox' is a typed tool field, written as 'tools.sandbox' on the seat.".to_string()
        }
        _ => String::new(),
    };
    Err(CompileError::Invalid(format!(
        "seat {} driver has an unknown key, {named}; known: {}.{place} A key the compiler \
         does not read is a declaration that was never made — a capability placed there would \
         compile, deliver nothing and run the seat at its harness default — so it is refused \
         rather than ignored (decision 0004; decision 0065 slice one, rebuild unit 5e)",
        bounded_site(what),
        DRIVER_KEYS.join(", ")
    )))
}

/// Decision 0046 ruling 1: the boundary is the realm's fact, declared in
/// `realms.json` under `forge.realms/v4`, and a bundle never names it. A
/// site that writes the key is told where the word lives rather than
/// that the key is unknown.
pub(super) fn refuse_boundary_key(what: &str, raw: &Value) -> Result<(), CompileError> {
    match raw.get("boundary") {
        None => Ok(()),
        Some(_) => Err(CompileError::Invalid(format!(
            "seat '{what}' declares boundary; {}",
            brokkr_protocol::hands::BOUNDARY_IS_THE_REALMS
        ))),
    }
}

/// Decision 0057 ruling 1: a crossing is the REALM's — a file one realm
/// publishes and another realm pins by its bytes — declared in
/// `realms.json` under `forge.realms/v5`, and a bundle never names one.
///
/// The same refusal `boundary` gets one function above, on the same terms
/// and for the same reason: a site that writes the word is told where the
/// word lives rather than that its key is unknown. Per seat, like
/// `boundary`, because a seat is the site that would claim the fact;
/// there is no root-level unknown-key check for either.
fn refuse_crossing_keys(what: &str, raw: &Value) -> Result<(), CompileError> {
    let Some(key) = CROSSING_KEYS.iter().find(|key| raw.get(*key).is_some()) else {
        return Ok(());
    };
    Err(CompileError::Invalid(format!(
        "seat '{what}' declares {key}; {CROSSING_IS_THE_REALMS}"
    )))
}

/// The two words `forge.realms/v5` adds, refused wherever a bundle writes
/// one.
const CROSSING_KEYS: [&str; 2] = ["publishes", "consumes"];

/// Where a crossing lives (decision 0057 ruling 1), said once for the
/// site that tries to write it into a bundle.
const CROSSING_IS_THE_REALMS: &str = "a crossing is declared by the realm \
    (realms.json, forge.realms/v5) and never by a bundle, because the file one realm \
    publishes and the digest another pins it at is the realm's fact and not a \
    recipe's (decision 0057 ruling 1)";

/// The keys a SEAT may write. Closed, like the class vocabulary itself
/// and for the same reason — see [`refuse_unknown_keys`].
const SEAT_KEYS: &[&str] = &[
    "results",
    "inputs",
    "limits",
    "secrets",
    "class",
    "agent",
    "role",
    "driver",
    "hands",
    "capabilities",
    "tools",
    "panel",
    "aggregate",
    "sequence",
    "select",
];

const BODY_KEYS: &[&str] = &[
    "class",
    "agent",
    "role",
    "driver",
    "hands",
    "capabilities",
    "tools",
    "panel",
    "aggregate",
    "sequence",
];

/// The keys a PANEL MEMBER may write. Narrower than a seat's: a member
/// has no `results`, `limits`, `inputs` or `secrets` of its own — the
/// seat above it does — which is why an agent declaring them at a member
/// site is already refused rather than silently discarded.
const MEMBER_KEYS: &[&str] = &[
    "class",
    "agent",
    "role",
    "driver",
    "hands",
    "capabilities",
    "tools",
];

/// The keys a SEQUENCE STEP may write: a member's, plus its name, plus
/// the two a step needs to be a panel of its own.
const STEP_KEYS: &[&str] = &[
    "name",
    "results",
    "class",
    "agent",
    "role",
    "driver",
    "hands",
    "capabilities",
    "tools",
    "panel",
    "aggregate",
    "dialect",
];

/// A `tools` declaration beside a panel, sequence or select is refused
/// (design D5.2): a local declaration belongs to the site that executes,
/// and a container that carried one could only share it as a grant or
/// drop it — so even an empty object is refused here.
fn refuse_tools_on_container(what: &str, raw: &Value) -> Result<(), CompileError> {
    match raw.get("tools") {
        None => Ok(()),
        Some(_) => Err(CompileError::Invalid(format!(
            "seat '{what}' declares 'tools' beside a panel, sequence or select; a local \
             declaration belongs to the site that executes — the member, step or case body — \
             and a container cannot own one, even an empty object, because it would either \
             become a shared grant or be ignored (decision 0065 slice one, design D5)"
        ))),
    }
}

/// The vocabulary of a site object is CLOSED, because since decision
/// 0021 a dropped key is a dropped refusal. `class` is read by absence —
/// an undeclared site is work — so `"clas": "gate"` would leave a
/// judging site classed work, and every gate refusal below it unarmed,
/// with nothing anywhere to say so. The same silence would swallow a
/// misspelled `secrets`. So a key this compiler does not read is refused
/// where it is written, in the manner of an unknown class or an unknown
/// aggregate: the fail-closed reading of an absent declaration is only
/// honest if an absence cannot be manufactured by a typo.
fn refuse_unknown_keys(what: &str, raw: &Value, known: &[&str]) -> Result<(), CompileError> {
    let Some(object) = raw.as_object() else {
        return Ok(());
    };
    for key in object.keys() {
        if !known.contains(&key.as_str()) {
            return Err(CompileError::Invalid(format!(
                "seat '{what}' has unknown key '{key}'; known: {}. The site \
                 vocabulary is closed because a declaration this compiler \
                 cannot see is a declaration that was never made — a \
                 misspelled 'class' would leave a gate reading as work \
                 (decision 0021 ruling 1)",
                known.join(", ")
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
