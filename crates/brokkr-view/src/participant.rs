//! What a participant's own record decides about reading its transcript:
//! whether a legacy Claude id may stand in for an absent reference,
//! whether it can still gain prose, and the subject a surface re-resolves.
//! Derived here once, so the terminal, the browser and `brokkr transcript`
//! cannot read one participant two ways (decision 0013; #351).

use crate::transcript::LegacyProvenance;
use crate::{Participant, Transcript, WORKING};

impl Participant {
    /// The legacy-synthesis rule this participant's provenance admits:
    /// only Claude, LaneTally and an inline seat with no provenance may
    /// fall back to a local Claude id. Codex and DSH provenance refuses
    /// synthesis.
    pub fn legacy_provenance(&self) -> LegacyProvenance {
        match self
            .provenance
            .as_ref()
            .map(|provenance| provenance.provider.as_str())
        {
            None => LegacyProvenance::Absent,
            Some("claude") => LegacyProvenance::Claude,
            Some("lanetally") => LegacyProvenance::LaneTally,
            Some(_) => LegacyProvenance::Other,
        }
    }

    /// Whether the participant is still at work, and so can still gain
    /// prose.
    pub fn working(&self) -> bool {
        self.status == WORKING.0
    }
}

/// The selected transcript subject: realm/journal identity, the full run
/// id, the participant key and the complete effective reference. A shell
/// re-resolves exactly this participant and no other, so no surface
/// gates the pane on a Claude session id or any other single kind.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Subject {
    /// The active hearth, so a stamp taken in one realm never speaks for
    /// the same run or participant name in another.
    pub tab: usize,
    /// The realm name when the world has tabs; the tab index alone is the
    /// journal identity in a one-hearth world.
    pub realm: Option<String>,
    /// The full run id, not a selector.
    pub run: String,
    /// The exact participant key.
    pub key: String,
    /// The complete recorded common reference, echoed even when it cannot
    /// be validated — a present reference always wins.
    pub reference: Option<Transcript>,
    /// How an absent common reference may synthesize a legacy Claude one.
    pub provenance: LegacyProvenance,
    /// The compatibility flat id a legacy synthesis may use.
    pub legacy_id: Option<String>,
    /// Whether the participant can still gain prose: the reader watches
    /// the source only while this holds.
    pub working: bool,
}

impl Subject {
    /// `part`'s subject in run `run`, read in hearth `tab` (named `realm`
    /// when the world has tabs), with every fact the shared reader needs.
    /// No field asks which kind it is, so a Codex thread or DSH session
    /// reaches the same read path as a Claude one.
    pub fn of(tab: usize, realm: Option<String>, run: String, part: &Participant) -> Subject {
        Subject {
            tab,
            realm,
            run,
            key: part.key.clone(),
            reference: part.transcript.clone(),
            provenance: part.legacy_provenance(),
            legacy_id: part.session_id.clone(),
            working: part.working(),
        }
    }
}
