//! Which site this is (#349): the one structural identity of a model
//! invocation site, built from the compiled body walk and never by
//! splitting a display tag on `:`. The resume digest is built from it
//! (proposed decision 0056), and the compiler's census and the verify
//! wrapper read the label and tag it renders.

use serde_json::{json, Value};

/// The domain separator. Two digests over two different structures must
/// never collide into each other, and a domain string is the cheapest
/// way to say so.
const SITE_DOMAIN: &str = "brokkr.resume-site/v1";

/// Where inside its seat one invocation site sits. The ancestry is
/// carried in distinct fields with their own indices, taken off the
/// compiled walk: `a:b`/`c` and `a`/`b:c` are two different values here,
/// which is exactly what the flattened display tag cannot say.
#[derive(Debug, Clone, PartialEq, Eq)]
enum SitePath {
    Single,
    PanelMember {
        member: String,
        member_index: usize,
    },
    SequenceStep {
        step: String,
        step_index: usize,
    },
    SequencePanelMember {
        step: String,
        step_index: usize,
        member: String,
        member_index: usize,
    },
}

impl SitePath {
    fn value(&self) -> Value {
        match self {
            SitePath::Single => json!({"kind": "single"}),
            SitePath::PanelMember {
                member,
                member_index,
            } => json!({
                "kind": "panel-member",
                "member": member,
                "member_index": member_index,
            }),
            SitePath::SequenceStep { step, step_index } => json!({
                "kind": "sequence-step",
                "step": step,
                "step_index": step_index,
            }),
            SitePath::SequencePanelMember {
                step,
                step_index,
                member,
                member_index,
            } => json!({
                "kind": "sequence-panel-member",
                "step": step,
                "step_index": step_index,
                "member": member,
                "member_index": member_index,
            }),
        }
    }
}

/// One structural invocation site: the outer seat, the selected case
/// (explicitly absent for a body with no selector), the body kind and
/// the full path. The run supplies its own scope — the journal this is
/// asked of is one run's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SiteKey {
    seat: String,
    /// `None` is an unselected body, and it is NOT the same key as a
    /// case literally named "default": absence is its own value here.
    case: Option<String>,
    body: &'static str,
    path: SitePath,
}

impl SiteKey {
    pub(crate) fn single(seat: &str, case: Option<&str>) -> SiteKey {
        SiteKey {
            seat: seat.to_string(),
            case: case.map(str::to_string),
            body: "single",
            path: SitePath::Single,
        }
    }

    pub(crate) fn panel_member(
        seat: &str,
        case: Option<&str>,
        member: &str,
        member_index: usize,
    ) -> SiteKey {
        SiteKey {
            seat: seat.to_string(),
            case: case.map(str::to_string),
            body: "panel",
            path: SitePath::PanelMember {
                member: member.to_string(),
                member_index,
            },
        }
    }

    pub(crate) fn sequence_step(
        seat: &str,
        case: Option<&str>,
        step: &str,
        step_index: usize,
    ) -> SiteKey {
        SiteKey {
            seat: seat.to_string(),
            case: case.map(str::to_string),
            body: "sequence",
            path: SitePath::SequenceStep {
                step: step.to_string(),
                step_index,
            },
        }
    }

    pub(crate) fn sequence_panel_member(
        seat: &str,
        case: Option<&str>,
        step: &str,
        step_index: usize,
        member: &str,
        member_index: usize,
    ) -> SiteKey {
        SiteKey {
            seat: seat.to_string(),
            case: case.map(str::to_string),
            body: "sequence",
            path: SitePath::SequencePanelMember {
                step: step.to_string(),
                step_index,
                member: member.to_string(),
                member_index,
            },
        }
    }

    /// Whether this site is the seat's own single body — the one shape
    /// decision 0030's legacy codex evidence can answer for, because a
    /// composite row's ancestry was never journaled.
    pub(crate) fn is_single(&self) -> bool {
        matches!(self.path, SitePath::Single)
    }

    fn value(&self) -> Value {
        json!({
            "domain": SITE_DOMAIN,
            "seat": self.seat,
            "case": self.case,
            "body": self.body,
            "path": self.path.value(),
        })
    }

    /// 64 lowercase hex, through the same canonical SHA-256 every other
    /// digest in this realm goes through.
    pub(crate) fn digest(&self) -> String {
        brokkr_core::canonical::sha256_hex(&self.value())
    }

    /// The site's tag inside its selected body, as the engine's maps key
    /// it: none for a single body, `<member>`, `<step>`, or
    /// `<step>:<member>`.
    pub(crate) fn tag(&self) -> Option<String> {
        match &self.path {
            SitePath::Single => None,
            SitePath::PanelMember { member, .. } => Some(member.clone()),
            SitePath::SequenceStep { step, .. } => Some(step.clone()),
            SitePath::SequencePanelMember { step, member, .. } => Some(format!("{step}:{member}")),
        }
    }

    /// The flattened address the compiled site facts are keyed by: the
    /// seat, then its case, then its tag, each present one joined by `:`.
    pub(crate) fn label(&self) -> String {
        let tag = self.tag();
        let parts = [
            Some(self.seat.as_str()),
            self.case.as_deref(),
            tag.as_deref(),
        ];
        parts.into_iter().flatten().collect::<Vec<_>>().join(":")
    }

    /// The site, named for an operator in a refusal.
    pub(crate) fn describe(&self) -> String {
        match &self.path {
            SitePath::Single => format!("the single body of seat '{}'", self.seat),
            SitePath::PanelMember { member, .. } => {
                format!("panel member '{member}' of seat '{}'", self.seat)
            }
            SitePath::SequenceStep { step, .. } => {
                format!("step '{step}' of seat '{}'", self.seat)
            }
            SitePath::SequencePanelMember { step, member, .. } => {
                format!("member '{member}' of step '{step}' of seat '{}'", self.seat)
            }
        }
    }
}

#[cfg(test)]
mod tests;
