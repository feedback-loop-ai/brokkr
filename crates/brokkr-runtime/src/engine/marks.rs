//! What the engine writes into one site's driver input beyond the seat's
//! own fields: the hands and boundary markers, the result door, the
//! discovery notice and the dialect prose. One home, so #342's
//! prompt-byte budget renders every site from the composition the engine
//! spawns it with rather than from a second copy of it (decision 0071
//! ruling 5).

use brokkr_core::realms::Boundary;
use brokkr_protocol::adapters::{HandsNotice, ACCESS};
use brokkr_protocol::native_controls::Access;
use serde_json::{json, Value};

use crate::agents::{Candidate, ResultDoor};
use crate::bundle::{Bundle, HandsState, McpContribution, McpIntent, SiteFacts};
use crate::capabilities::{Holding, Implementation, Outcome};

/// The private carrier of the MCP servers the serving candidate is given
/// beside its hands (U7c; MB1, D8): written and cleared by the engine
/// alone, outside the requested digest, for U7d's guidance to render.
pub(super) const SELECTED_SERVERS: &str = "capability_servers";

/// One MCP holding the serving candidate's own intended set brokers
/// (U7c; MB1): its capability, its `cap-<capability>` server and the
/// holding sealed at compile.
pub(super) struct Brokered<'a> {
    pub(super) capability: &'a str,
    pub(super) server: &'a str,
    pub(super) holding: &'a Holding,
}

/// The marks of one compiled bundle under one boundary: the engine's own
/// field, which every run it drives takes from the bundle.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct SiteMarks<'a> {
    pub bundle: &'a Bundle,
    pub boundary: Boundary,
}

impl<'a> SiteMarks<'a> {
    /// Every spawn-time mark of one model site, in the engine's order:
    /// hands and the access they serve, then the result door, then the
    /// discovery notice, then what the serving candidate holds (decision
    /// 0065), then the MCP servers it is given.
    pub fn site(&self, label: &str, gate: bool, link: Option<&Candidate>, input: &mut Value) {
        self.hands(label, input);
        self.access(label, gate, input);
        self.door(label, gate, link, input);
        self.notice(label, link, input);
        self.capabilities(label, link, input);
        self.servers(label, link, input);
    }

    /// The outcome compiled for the candidate serving `label`: the selected
    /// link's own, or an inline site's one.
    fn outcome(&self, label: &str, link: Option<&Candidate>) -> Option<&'a Outcome> {
        outcome_at(self.bundle.sites.get(label), link)
    }

    /// The MCP holdings the serving candidate's own intended set brokers
    /// (U7c; MB1): the holding behind each server the set that candidate
    /// composed with contributes beside the box's hands
    /// ([`McpContribution::Brokered`]), in its order, and none otherwise.
    /// The one projection the plans the engine provisions, the brokers it
    /// seals and the servers it names to the seat are all read from
    /// ([`brokered_at`]).
    pub(super) fn brokered(&self, label: &str, link: Option<&Candidate>) -> Vec<Brokered<'a>> {
        brokered_at(self.bundle.sites.get(label), link)
    }

    /// The MCP servers the serving candidate is given (U7c; MB1, D8): each
    /// brokered holding's server, capability and exact tools, and the
    /// discovery tool of the notice that candidate's own adapter declares,
    /// which U7d renders. Cleared first, as `notice` clears its carrier, so
    /// a reused input, a fallback or a dropped wanted holding never keeps
    /// one it does not own; written only where a holding is brokered. It
    /// names no plan, store or ledger locator.
    pub(super) fn servers(&self, label: &str, link: Option<&Candidate>, input: &mut Value) {
        // Every driver input the engine composes is an object.
        let _ = input
            .as_object_mut()
            .map(|object| object.remove(SELECTED_SERVERS));
        let brokered = self.brokered(label, link);
        if brokered.is_empty() {
            return;
        }
        let servers: Vec<Value> = (brokered.iter())
            .map(|held| {
                json!({"server": held.server, "capability": held.capability,
                       "tools": held.holding.tools})
            })
            .collect();
        let discovery = self.declared_notice(label, link);
        input[SELECTED_SERVERS] = json!({
            "servers": servers,
            "discovery_tool": discovery.map(HandsNotice::discovery_tool),
        });
    }

    /// Decision 0065: the native controls composed for the candidate
    /// serving this attempt, and the prompt's statement of what it holds —
    /// a fallback gets its own, never its primary's. Written here, the one
    /// helper #342's prompt-byte budget renders through, so the budget
    /// measures the capabilities text a seat is actually handed (operator
    /// ruling 2026-09-29, decision 0071 ruling 5). A site with no computed
    /// outcome gets an explicit `null`, which the model adapters refuse
    /// rather than launching a harness on its own defaults.
    pub fn capabilities(&self, label: &str, link: Option<&Candidate>, input: &mut Value) {
        let outcome = self.outcome(label, link);
        input["native_controls"] = outcome.map_or(Value::Null, |outcome| outcome.controls());
        input["capabilities"] = outcome.map_or(Value::Null, |outcome| outcome.prompt());
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

    /// The hands access a boxed site serves (#896 round 2): the office's
    /// own `read-only` or `writes`, carried to this site with its hands.
    /// An inline site has no office to declare one, so its gate class
    /// stands in exactly as it did; a changed office declaration now moves
    /// the mode rather than being re-derived from the phase's class.
    /// Cleared first, as `notice` clears its carrier, and written only where
    /// this label resolves hands and Brokkr boxes them; a spawn-time fact,
    /// outside the requested digest.
    pub(super) fn access(&self, label: &str, gate: bool, input: &mut Value) {
        // Every driver input the engine composes is an object.
        let _ = input.as_object_mut().map(|object| object.remove(ACCESS));
        if !(self.has_hands(label) && self.boundary.is_boxed()) {
            return;
        }
        let declared = self.bundle.sites.get(label).and_then(|facts| facts.access);
        let access = declared.unwrap_or(match gate {
            true => Access::ReadOnly,
            false => Access::Writes,
        });
        input[ACCESS] = serde_json::to_value(access).expect("an access is a scalar word");
    }

    /// The judge's door under `harness` (decision 0046 ruling 4; design
    /// D23): a gate-class site with hands whose selected link declares
    /// `hands.harness.result` as `last-message` is told so, because its
    /// final message — not a file it writes — is what reaches the engine.
    /// A spawn-time fact of the selected link, written after the
    /// requested digest was checked: a chain fallback moves the door, and
    /// a digest that moved with it would refuse the retry as a different
    /// effect. Rebuild unit 5d (operator ruling of 2026-09-25): an inline
    /// Codex gate whose read-only class the engine lowered is told so too;
    /// [`super::result_door`] is the one answer for both.
    pub(super) fn door(
        &self,
        label: &str,
        gate: bool,
        link: Option<&Candidate>,
        input: &mut Value,
    ) {
        let facts = self.bundle.sites.get(label);
        if super::result_door(self.boundary, gate, facts, link) == ResultDoor::LastMessage {
            input["result_delivery"] = json!("last-message");
        }
    }

    /// The discovery notice (decision 0069): a boxed seat whose
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
        if let Some(notice) = self.declared_notice(label, link) {
            input["hands_notice"] = notice.to_value();
        }
    }

    /// The notice the provider serving this attempt declares: the selected
    /// link's, or — only for an inline site no link serves — the one its
    /// adapter declared at compile time.
    fn declared_notice<'n>(
        &'n self,
        label: &str,
        link: Option<&'n Candidate>,
    ) -> Option<&'n HandsNotice> {
        match link {
            Some(link) => link.hands_notice.as_ref(),
            None => {
                (self.bundle.sites.get(label)).and_then(|facts| facts.inline_hands_notice.as_ref())
            }
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

/// The outcome compiled for the candidate serving the site `facts`
/// describe: the selected link's own, or an inline site's one.
pub(super) fn outcome_at<'a>(
    facts: Option<&'a SiteFacts>,
    link: Option<&Candidate>,
) -> Option<&'a Outcome> {
    let site = facts?.capabilities.as_ref()?;
    site.serving(link.map(|link| (link.provider.as_str(), link.model.as_str())))
}

/// The MCP holdings the serving candidate's own intended set brokers at the
/// site `facts` describe ([`SiteMarks::brokered`]).
pub(super) fn brokered_at<'a>(
    facts: Option<&'a SiteFacts>,
    link: Option<&Candidate>,
) -> Vec<Brokered<'a>> {
    let intent = match link {
        Some(link) => McpIntent::of_candidate(link),
        None => facts.and_then(|facts| facts.inline_mcp),
    };
    let Some(outcome) = outcome_at(facts, link) else {
        return Vec::new();
    };
    let servers = match intent.map(|intent| intent.contribution(outcome)) {
        Some(Ok(McpContribution::Brokered(servers))) => servers,
        // An `Unservable` set brokers nothing here because the compile
        // already refused it (`McpSite::contribute`), which
        // `bundle::agent_tests::mcp_tests::a_set_without_the_hands_server_or_a_model_channel_carries_no_broker`
        // binds as this arm's removal control.
        Some(Ok(
            McpContribution::Empty | McpContribution::Hands | McpContribution::NoModelSurface,
        ))
        | Some(Err(_))
        | None => Vec::new(),
    };
    let held = |server: &str| {
        (outcome.held.iter()).find_map(|(capability, holding)| match &holding.implementation {
            Implementation::Mcp { server: named, .. } if named == server => Some(Brokered {
                capability,
                server: named,
                holding,
            }),
            Implementation::Mcp { .. } | Implementation::Native { .. } => None,
        })
    };
    servers.into_iter().filter_map(held).collect()
}

#[cfg(test)]
mod tests;
