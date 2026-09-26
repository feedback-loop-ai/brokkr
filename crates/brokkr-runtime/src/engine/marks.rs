//! What the engine writes into one site's driver input beyond the seat's
//! own fields: the hands and boundary markers, the result door, the
//! discovery notice and the dialect prose. One home, so #342's
//! prompt-byte budget renders every site from the composition the engine
//! spawns it with rather than from a second copy of it (decision 0071
//! ruling 5).

use brokkr_core::realms::Boundary;
use serde_json::{json, Value};

use crate::agents::{Candidate, ResultDoor};
use crate::bundle::{Bundle, HandsState};

/// The marks of one compiled bundle under one boundary: the engine's own
/// field, which every run it drives takes from the bundle.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct SiteMarks<'a> {
    pub bundle: &'a Bundle,
    pub boundary: Boundary,
}

impl SiteMarks<'_> {
    /// Every spawn-time mark of one model site, in the engine's order:
    /// hands, then the result door, then the discovery notice.
    pub fn site(&self, label: &str, gate: bool, link: Option<&Candidate>, input: &mut Value) {
        self.hands(label, input);
        self.door(label, gate, link, input);
        self.notice(label, link, input);
    }

    /// Decision 0043: a boxed site is told that it is, because the one
    /// tool the box serves is the only thing that can write its result
    /// file — a harness's own shell runs outside the box and a file
    /// written through it never reaches the engine. The first
    /// astra-judged gate wrote its verdict through that shell twice.
    ///
    /// Decision 0046 (design DD21): the same mark, grown into the one
    /// helper that writes the boundary into every site with hands —
    /// `boundary`, the realm's word, under every boundary; `hands: boxed`
    /// only when Brokkr builds the box, so the marker is never a false
    /// statement under `harness` or `open`, where no workspace tool is
    /// served. A site without hands is untouched.
    pub(super) fn hands(&self, label: &str, input: &mut Value) {
        match self.bundle.sites.get(label).map(|facts| &facts.hands) {
            Some(HandsState::Hands(_)) => {
                input["boundary"] = json!(self.boundary.word());
                input["hands"] = json!(if self.boundary.is_boxed() {
                    "boxed"
                } else {
                    "none"
                });
            }
            // A registered, resolved no-hands site is an affirmative
            // fact: the adapter gate requires it rather than reading the
            // absence as permission (design D10 F1).
            Some(HandsState::NoHands) => {
                input["boundary"] = json!("not applicable");
                input["hands"] = json!("none");
            }
            // Unknown is not `none`. An unregistered or unresolved site
            // publishes no affirmative marker, so the adapter declines.
            Some(HandsState::Unknown) | None => {
                input["boundary"] = Value::Null;
                input["hands"] = Value::Null;
            }
        }
    }

    /// The judge's door under `harness` (decision 0046 ruling 4; design
    /// D23): a gate-class site with hands whose selected link declares
    /// `hands.harness.result` as `last-message` is told so, because its
    /// final message — not a file it writes — is what reaches the engine.
    /// A spawn-time fact of the selected link, written after the
    /// requested digest was checked: a chain fallback moves the door, and
    /// a digest that moved with it would refuse the retry as a different
    /// effect.
    pub(super) fn door(
        &self,
        label: &str,
        gate: bool,
        link: Option<&Candidate>,
        input: &mut Value,
    ) {
        let door = link.map(|link| link.harness.result);
        if self.boundary == Boundary::Harness
            && gate
            && self.has_hands(label)
            && door == Some(ResultDoor::LastMessage)
        {
            input["result_delivery"] = json!("last-message");
        }
    }

    /// The discovery notice (proposed decision 0069): a boxed seat whose
    /// provider may defer MCP tools is told which tool is its workspace
    /// and how to load it. The engine alone writes the private
    /// `hands_notice` carrier, and clears it first, so a reused input, a
    /// fallback to a provider that declares none, or a site that is not
    /// boxed never keeps a carrier it does not own. It is written only
    /// when this label's canonical facts resolve hands, the boundary is
    /// one Brokkr boxes, and the provider serving THIS attempt declares a
    /// notice: the selected link's, or — only for an inline site no link
    /// serves — the notice its adapter declared at compile time. A spawn
    /// time fact of the selected link, like `door`'s, so it
    /// stays outside the requested digest.
    pub(super) fn notice(&self, label: &str, link: Option<&Candidate>, input: &mut Value) {
        // Every driver input the engine composes is an object.
        let _ = input
            .as_object_mut()
            .map(|object| object.remove("hands_notice"));
        if !(self.has_hands(label) && self.boundary.is_boxed()) {
            return;
        }
        let notice = match link {
            Some(link) => link.hands_notice.as_ref(),
            None => self
                .bundle
                .sites
                .get(label)
                .and_then(|facts| facts.inline_hands_notice.as_ref()),
        };
        if let Some(notice) = notice {
            input["hands_notice"] = notice.to_value();
        }
    }

    /// Whether the one canonical execution-site family resolved any hands
    /// at this label (design D10 F1). `NoHands` and `Unknown` both answer
    /// false; `hands` tells those two apart with affirmative markers
    /// rather than letting this answer invent one.
    pub(super) fn has_hands(&self, label: &str) -> bool {
        matches!(
            self.bundle.sites.get(label).map(|facts| &facts.hands),
            Some(HandsState::Hands(_))
        )
    }

    /// The dialect prose a seat's own input carries: its phase's, outside
    /// review, and for implement only once a change is named for it to
    /// implement. Review's prose reaches its spec-compliance member alone
    /// (see [`SiteMarks::member_dialect`]).
    pub fn seat_dialect(&self, phase: &str, has_change: bool, input: &mut Value) {
        if phase != "review" && (phase != "implement" || has_change) {
            if let Some(instructions) = self.bundle.dialect_prompts.get(phase) {
                input["spec_dialect"] = json!(instructions);
            }
        }
    }

    /// The dialect prose one panel member reads: review's own for review's
    /// spec-compliance member, and otherwise the prose its seat carries.
    pub fn member_dialect(&self, member: &str, seat_input: &Value, input: &mut Value) {
        if seat_input["phase"] == "review" && member == "spec-compliance" {
            input["spec_dialect"] = self
                .bundle
                .dialect_prompts
                .get("review")
                .map_or(Value::Null, |text| json!(text));
        } else if !seat_input["spec_dialect"].is_null() {
            input["spec_dialect"] = seat_input["spec_dialect"].clone();
        }
    }
}
