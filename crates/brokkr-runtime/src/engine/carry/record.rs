//! Optional panel evidence; result values remain opaque until aggregation (0093 ruling 4).
use std::collections::BTreeMap;
use std::path::Path;

use brokkr_protocol::{AttemptOutcome, AttemptReport};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_store::{SeatRecordError, StoreError};

use super::super::panel::Stamped;
use super::super::worktree::{read_tree, TreeReading};
use super::super::{git_head, is_commit_id, Engine, EngineError};
use crate::bundle::MemberFindings;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct Tag(pub(crate) String);

impl Tag {
    pub(crate) fn new(step: Option<&str>, member: &str) -> Self {
        let prefix = step.map(|step| format!("{step}:")).unwrap_or_default();
        Self::prefixed(&prefix, member)
    }

    pub(crate) fn prefixed(prefix: &str, member: &str) -> Self {
        Self(format!("{prefix}{member}"))
    }

    pub(crate) fn bare<'a>(&'a self, step: Option<&str>) -> &'a str {
        step.and_then(|step| self.0.strip_prefix(&format!("{step}:")))
            .unwrap_or(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub(crate) struct Head(String);

impl TryFrom<String> for Head {
    type Error = &'static str;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        is_commit_id(&value)
            .then_some(Self(value))
            .ok_or("expected forty lowercase hex digits")
    }
}

impl Head {
    pub(crate) fn observe(repo: &Path) -> Option<Self> {
        git_head(repo).and_then(|head| head.try_into().ok())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Ends<T> {
    pub(crate) at_start: T,
    pub(crate) at_end: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PanelSpan {
    pub(crate) tree: Ends<TreeReading>,
    #[serde(
        default,
        skip_serializing_if = "heads_absent",
        serialize_with = "write_heads",
        deserialize_with = "read_heads"
    )]
    pub(crate) head: Ends<Option<Head>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordedHeads {
    #[serde(default, deserialize_with = "head_end")]
    at_start: Option<Head>,
    #[serde(default, deserialize_with = "head_end")]
    at_end: Option<Head>,
}

fn head_end<'de, D: Deserializer<'de>>(de: D) -> Result<Option<Head>, D::Error> {
    Head::deserialize(de).map(Some)
}

pub(crate) fn read_heads<'de, D: Deserializer<'de>>(de: D) -> Result<Ends<Option<Head>>, D::Error> {
    let heads = RecordedHeads::deserialize(de)?;
    Ok(Ends {
        at_start: heads.at_start,
        at_end: heads.at_end,
    })
}

fn write_heads<S: Serializer>(
    heads: &Ends<Option<Head>>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let mut values = BTreeMap::new();
    if let Some(head) = &heads.at_start {
        values.insert("at_start", head);
    }
    if let Some(head) = &heads.at_end {
        values.insert("at_end", head);
    }
    values.serialize(serializer)
}

pub(crate) fn parse_heads(value: Value) -> Result<Ends<Option<Head>>, serde_json::Error> {
    read_heads(value)
}

fn heads_absent(head: &Ends<Option<Head>>) -> bool {
    head.at_start.is_none() && head.at_end.is_none()
}

impl PanelSpan {
    pub(crate) fn open(repo: &Path) -> Self {
        let tree = read_tree(repo);
        let head = Head::observe(repo);
        Self {
            tree: Ends {
                at_start: tree,
                at_end: tree,
            },
            head: Ends {
                at_start: head.clone(),
                at_end: head,
            },
        }
    }

    pub(crate) fn close(&mut self, repo: &Path) {
        self.tree.at_end = read_tree(repo);
        self.head.at_end = Head::observe(repo);
    }

    pub(crate) fn holds(&self, admitted: &Head) -> bool {
        self.head.at_start.as_ref() == Some(admitted)
            && self.head.at_end.as_ref() == Some(admitted)
            && self.tree.at_start == TreeReading::Clean
            && self.tree.at_end == TreeReading::Clean
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PanelRecord {
    #[serde(flatten)]
    pub(crate) span: PanelSpan,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) settled_members: BTreeMap<Tag, Value>,
}

impl PanelRecord {
    pub(crate) fn stamp(&self, payload: &mut Value) {
        let record = serde_json::to_value(self).expect("panel evidence serializes");
        payload
            .as_object_mut()
            .expect("terminal payload object")
            .extend(record.as_object().expect("record object").clone());
    }
}

/// Each member that succeeded, by its tag, with the result the panel's
/// record journals: its payload carrying the findings it was admitted
/// with, written here at the journal's boundary.
pub(crate) fn settled_members(
    (reports, stamped): (&[(String, AttemptReport)], &Stamped),
    step: Option<&str>,
) -> BTreeMap<Tag, Value> {
    reports
        .iter()
        .filter_map(|(name, report)| match report.settled_outcome() {
            AttemptOutcome::Succeeded { mut result } => {
                if let Some(MemberFindings::Read(findings)) = stamped.get(name) {
                    brokkr_core::findings::journal(&mut result, findings);
                }
                Some((Tag::new(step, name), result))
            }
            AttemptOutcome::Failed { .. } | AttemptOutcome::Indeterminate { .. } => None,
        })
        .collect()
}

impl Engine {
    /// `payload` appended under the seat-record fence. A panel's failed or
    /// indeterminate terminal whose marked settled member's payload the
    /// fence refuses (fix round 0109a1r6, W3) carries no settled verdict
    /// for that member: it is journaled again without that member alone,
    /// the refusal named in its own words, as a refused success is
    /// journaled as a failure, and every sibling the fence admits keeps its
    /// payload, verdict and retry eligibility (fix round 0109a1r7, X4). The
    /// refused member is then unsettled, so a retry runs it fresh.
    ///
    /// The refusal names its member by its exact tag, as data, and the
    /// turns are bounded by the payload's own settled members (fix round
    /// 0109a1r8, M4): each turn unsettles one, so once every member is
    /// gone the last append is the fence's own word, whatever any tag
    /// holds.
    pub(in crate::engine) fn append_settled(
        &mut self,
        event_type: EventType,
        mut payload: Value,
        attempt_id: Option<String>,
    ) -> Result<EventEnvelope, EngineError> {
        let settled = (payload.get("settled_members").and_then(Value::as_object))
            .map_or(0, serde_json::Map::len);
        for _ in 0..settled {
            let appended = self.append_raw(event_type, payload.clone(), attempt_id.clone());
            let refused = match &appended {
                Err(EngineError::Store(StoreError::SeatRecord(refusal))) => refusal,
                _ => return appended,
            };
            let Some(tag) = refused.settled_member.as_deref() else {
                return appended;
            };
            unsettle(&mut payload, tag, refused);
        }
        self.append_raw(event_type, payload, attempt_id)
    }
}

/// `payload` with the settled member `tag` the fence refused removed, and
/// the refusal named by member in the terminal's own words, `error` or
/// `reason`.
fn unsettle(payload: &mut Value, tag: &str, refusal: &SeatRecordError) {
    let fields = payload
        .as_object_mut()
        .expect("a terminal payload is an object");
    let settled = (fields["settled_members"].as_object_mut())
        .expect("a refused settled member's terminal settles members");
    settled.remove(tag);
    if settled.is_empty() {
        fields.remove("settled_members");
    }
    let said = ["error", "reason"]
        .into_iter()
        .find(|field| fields.contains_key(*field));
    let said = said.expect("a failed or indeterminate terminal says why");
    let why = fields[said].as_str().unwrap_or_default();
    fields[said] = Value::from(format!(
        "{why}; settled member '{tag}' was refused: {refusal}"
    ));
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CarriedEntry {
    pub(crate) member: Tag,
    pub(crate) attempt_id: String,
    /// The carried member's site, from the engine's own marker in its
    /// source attempt, so a reader files the member's findings under it in
    /// the retry's effect too, never from the payload (fix round 0109a1r7,
    /// X2). Absent where no marker named one, as in every earlier journal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) site_ref: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RetryArgs {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "members"
    )]
    pub(crate) members: Option<Vec<Tag>>,
}

fn members<'de, D: Deserializer<'de>>(de: D) -> Result<Option<Vec<Tag>>, D::Error> {
    let members = Vec::<Tag>::deserialize(de)?;
    if members.is_empty() {
        return Err(serde::de::Error::custom("members must not be empty"));
    }
    Ok(Some(members))
}
