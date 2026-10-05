//! What a gate may hold (decision 0065 slice two, GP1), and the one typed
//! cause every lost ask carries.
//!
//! A gate IS the check (decision 0021 ruling 1), so the abstract classes
//! of what it holds are judged at the executable site's own canonical
//! class and stable office — never a container's, an execution label's or
//! a dialect's. The check is pure: it reads the site, the abstract
//! definition and the realm's grant, and runs after the grant's presence,
//! office reach and tool set were proved and before any provider carries
//! it (design D3 steps 3, 5 and 6), whatever kind the grant's dialect is.

use brokkr_core::realms::CapabilityGrant;
use brokkr_protocol::native_controls::Site;

use super::{Definition, SiteAsks};
use crate::bundle::SeatClass;

/// The abstract classes GP1 restricts at a gate. `reads` restricts
/// nothing, so it is not a case here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    Writes,
    Egress,
}

impl Class {
    /// The word a definition writes for the class (its closed schema).
    fn word(self) -> &'static str {
        match self {
            Class::Writes => "writes",
            Class::Egress => "egress",
        }
    }
}

/// Why a gate cannot hold a capability its office was otherwise granted.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(super) enum GateRefusal {
    /// Whatever else the abstraction is, and whoever the realm names.
    #[error("gate offices cannot hold a capability with class 'writes'")]
    Writes,
    /// A grant whose offices list omits the office, or is absent.
    #[error(
        "a gate's egress capability requires the realm grant to name office '{office}' explicitly"
    )]
    Egress { office: String },
}

/// GP1's one check. A work site is not judged here. A gate is refused any
/// `writes` abstraction first, then an `egress` one its grant does not
/// name its office for explicitly; an absent offices list reaches every
/// office (design D4) and still names none.
pub(super) fn check(
    site: &SiteAsks,
    definition: &Definition,
    grant: &CapabilityGrant,
) -> Result<(), GateRefusal> {
    let has = |class: Class| definition.classes.iter().any(|word| word == class.word());
    let named = grant
        .offices
        .as_deref()
        .is_some_and(|offices| offices.contains(&site.office));
    match site.class {
        SeatClass::Work => Ok(()),
        SeatClass::Gate if has(Class::Writes) => Err(GateRefusal::Writes),
        SeatClass::Gate if has(Class::Egress) && !named => Err(GateRefusal::Egress {
            office: site.office.clone(),
        }),
        SeatClass::Gate => Ok(()),
    }
}

/// Why an ask is not held. Whether a native OFF stands behind the loss is
/// NOT decided here: only the candidate's native plan knows what was
/// switched off, and a provider whose inventory is unmeasured denies
/// nothing it can show.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Cause {
    /// No grant reaches the office, so no dialect was ever chosen.
    Ungranted(String),
    /// The grant's dialect cannot carry it on this candidate (design D4).
    Incompatible { dialect: String, but: String },
    /// The grant reaches the office, and the site's class forbids it.
    Gate {
        dialect: String,
        refusal: GateRefusal,
    },
}

impl Cause {
    /// What the grant's `dialect` cannot carry on this candidate.
    pub(super) fn incompatible(dialect: &str, but: String) -> Cause {
        let dialect = dialect.to_string();
        Cause::Incompatible { dialect, but }
    }

    /// What GP1 refuses a site the grant's `dialect` would otherwise serve.
    pub(super) fn gate(dialect: &str, refusal: GateRefusal) -> Cause {
        let dialect = dialect.to_string();
        Cause::Gate { dialect, refusal }
    }

    /// ` through dialect '<dialect>'`, or nothing where no grant was found.
    pub(super) fn through(&self) -> String {
        match self {
            Cause::Ungranted(_) => String::new(),
            Cause::Incompatible { dialect, .. } | Cause::Gate { dialect, .. } => {
                format!(" through dialect '{dialect}'")
            }
        }
    }

    /// The reason the manifest and the prompt give.
    pub(super) fn but(&self) -> String {
        match self {
            Cause::Ungranted(but) | Cause::Incompatible { but, .. } => but.clone(),
            Cause::Gate { refusal, .. } => refusal.to_string(),
        }
    }

    /// The complete refusal of a `requires` ask. GP1's form closes on its
    /// cause; an incompatible grant says the grant cannot carry it.
    pub(super) fn required(&self, who: Site<'_>, capability: &str) -> String {
        let (through, but) = (self.through(), self.but());
        match self {
            Cause::Ungranted(_) => format!("{who}: requires capability '{capability}' but {but}"),
            Cause::Incompatible { .. } => format!(
                "{who}: requires capability '{capability}'{through}, but {but}; the capability \
                 cannot be held under this grant"
            ),
            Cause::Gate { .. } => {
                format!("{who}: requires capability '{capability}'{through}, but {but}")
            }
        }
    }
}
