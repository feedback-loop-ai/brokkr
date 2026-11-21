//! A panel's aggregate (#349), in one place: the word a bundle writes for
//! it, its parse and closed vocabulary, the results it can emit, and the
//! aggregation the engine runs over the members' results.

use std::collections::BTreeMap;

use brokkr_core::findings::{bind, Finding, ResidualInput, ResidualPair, WordRefusal};
use brokkr_core::policy::SEVERITY_ORDER;
use serde::Deserialize;
use serde_json::{json, Map, Value};

use super::{CompileError, PanelMember};

/// Deterministic, order-independent aggregation rules — a closed
/// vocabulary, like conditions: named in data, implemented in the
/// engine, never arbitrary code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aggregate {
    /// "pass" only when every member reports "pass"; otherwise "fail".
    UnanimousPass,
    /// Worst-member-wins over clean < residual < security-hold; severity
    /// is the max, security and fixes flags are OR-ed.
    ReviewPanel,
}

impl Aggregate {
    /// Every aggregate, in the order its vocabulary is named.
    const ALL: [Aggregate; 2] = [Aggregate::UnanimousPass, Aggregate::ReviewPanel];

    /// The marker a panel emits for a member payload with no readable
    /// result string: the aggregate's fail-closed poison, never a result
    /// a seat or step may declare (decision 0050, ruling 7).
    pub(super) const MEMBER_SCHEMA_INVALID: &'static str = "__member-schema-invalid__";

    /// The word a bundle writes for this aggregate, and its manifest
    /// records.
    pub(super) fn word(self) -> &'static str {
        match self {
            Aggregate::UnanimousPass => "unanimous-pass",
            Aggregate::ReviewPanel => "review-panel",
        }
    }

    /// The aggregate a bundle names by `word`, if it names one.
    pub(super) fn parse(word: &str) -> Option<Aggregate> {
        Aggregate::ALL
            .into_iter()
            .find(|aggregate| aggregate.word() == word)
    }

    /// The closed vocabulary, as a refusal names it.
    pub(super) fn known() -> String {
        Aggregate::ALL.map(Aggregate::word).join(", ")
    }

    /// Refuse a declared vocabulary that names
    /// [`MEMBER_SCHEMA_INVALID`](Self::MEMBER_SCHEMA_INVALID). The marker
    /// is the aggregate's poison for a malformed member, not a result a
    /// step can report, so refusing it where the vocabulary is declared
    /// keeps `emits` honest and a table arm for it unreachable.
    pub(super) fn refuse_member_marker(
        declared: &[String],
        what: &str,
    ) -> Result<(), CompileError> {
        if declared
            .iter()
            .any(|word| word == Self::MEMBER_SCHEMA_INVALID)
        {
            return Err(CompileError::Invalid(format!(
                "{what} may not declare '{}': a malformed panel member's marker is \
                 never a reportable result",
                Self::MEMBER_SCHEMA_INVALID
            )));
        }
        Ok(())
    }

    /// The results a declaring vocabulary must cover: every word the
    /// aggregate is required to be able to return. For `unanimous-pass`
    /// that is its own two words; for `review-panel` it is the three
    /// words the panel's ranking names, which a declared vocabulary may
    /// widen (a member's word is returned verbatim).
    pub(super) fn required_results(&self) -> &'static [&'static str] {
        match self {
            Aggregate::UnanimousPass => &["pass", "fail"],
            Aggregate::ReviewPanel => &["clean", "residual", "security-hold"],
        }
    }

    /// The table words [`Aggregate::aggregate_results`] can emit for a
    /// step whose declared results are `declared`, and which the step's
    /// declared-results check admits. `unanimous-pass` emits exactly
    /// `pass`/`fail`, which is all `unanimous_pass` can return for valid
    /// members. `review-panel` returns the worst member's word verbatim,
    /// so it emits every word the step DECLARES, including one beyond the
    /// aggregate's required results.
    ///
    /// A member payload with no readable result string becomes the
    /// [`MEMBER_SCHEMA_INVALID`](Self::MEMBER_SCHEMA_INVALID) marker,
    /// which [`refuse_member_marker`](Self::refuse_member_marker) refuses
    /// wherever a vocabulary is declared, so no panel emits it to the
    /// table. A `review-panel` member word the step does not declare is
    /// likewise not emittable: at a non-final step `vocabulary_problem`
    /// fails the attempt (0006-retryable), and only the final step's
    /// word reaches decide(), whose declared-results check parks it.
    pub(super) fn emits<'a>(&self, declared: &'a [String]) -> Vec<&'a str> {
        match self {
            Aggregate::UnanimousPass => vec!["pass", "fail"],
            Aggregate::ReviewPanel => declared.iter().map(String::as_str).collect(),
        }
    }

    /// A member's compiled result set (decision 0109 ruling 11): a
    /// `unanimous-pass` member keeps the aggregate's own two words
    /// whatever its agent declares; a `review-panel` member is held to its
    /// hired agent's `declared` list, or, declaring none, to the narrow
    /// `clean` and `residual`, never to the owning seat's set.
    pub(crate) fn member_results(self, declared: Option<Vec<String>>) -> Vec<String> {
        let words = |words: &[&str]| words.iter().map(|word| word.to_string()).collect();
        match self {
            Aggregate::UnanimousPass => words(&["pass", "fail"]),
            Aggregate::ReviewPanel => declared.unwrap_or_else(|| words(&["clean", "residual"])),
        }
    }
}

/// What one panel member's word answers to, compiled at its site: the
/// results it may report (decision 0109 ruling 11), and the residual pair
/// its owning seat compiles, which its word is bound to (ruling 2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MemberLaw {
    pub(crate) results: Vec<String>,
    pub(crate) residual: ResidualPair,
}

impl MemberLaw {
    /// A law admitting every word a review panel ranks, over a seat that
    /// declares no residual pair, for tests that build members by hand and
    /// do not test the law.
    #[cfg(test)]
    pub(crate) fn open() -> Self {
        Self::declaring(&["clean", "residual", "security-hold"])
    }

    /// A law admitting exactly `results`, over a seat that declares no
    /// residual pair.
    #[cfg(test)]
    pub(crate) fn declaring(results: &[&str]) -> Self {
        MemberLaw {
            results: results.iter().map(|word| word.to_string()).collect(),
            residual: ResidualPair::declared_in::<&str>(&[]),
        }
    }
}

/// A body the engine can never take, refused at compile rather than at
/// every run-time pass (decision 0109 follow-ups, #1221).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BodyRefusal {
    /// A `unanimous-pass` panel writes no residual pair, so under a seat
    /// that declares one its `pass` fails ruling 2's aggregate arm on
    /// every pass (decision 0109 ruling 2). Only `review-panel` joins it.
    #[error(
        "panel '{panel}' joins by 'unanimous-pass', which writes no residual pair, under a \
         seat whose input set declares '{}' or '{}'; its pass could never stand (decision 0109 \
         ruling 2): join it by 'review-panel' or drop the pair from the seat's inputs",
        ResidualInput::MaxResidualSeverity.name(),
        ResidualInput::HasSecurityResidual.name()
    )]
    UnanimousPassUnderResidual { panel: String },
    /// A protected sequence's step before its first gate step may end the
    /// sequence on a word that does not stop the run. That ending routes
    /// onward with no gate record, so decide refuses it on every pass
    /// (decision 0109 ruling 7, #1212 L1). A hard word there is a stop.
    #[error(
        "seat '{seat}' is the protected phase's, and its sequence step '{step}' runs before its \
         first gate step yet may end the sequence on '{word}', a word that does not stop the \
         run; no gate has recorded a head there, so the engine can never take that ending \
         (decision 0109 ruling 7)"
    )]
    PreGateEnding {
        seat: String,
        step: String,
        word: String,
    },
}

impl Aggregate {
    /// Refuse a `unanimous-pass` panel at `panel` under a seat whose
    /// compiled input set declares any of the residual pair.
    pub(super) fn refuse_unbindable(
        self,
        panel: &str,
        residual: ResidualPair,
    ) -> Result<(), BodyRefusal> {
        let undeclared = ResidualPair::declared_in::<&str>(&[]);
        match self {
            Aggregate::UnanimousPass if residual != undeclared => {
                Err(BodyRefusal::UnanimousPassUnderResidual {
                    panel: panel.to_string(),
                })
            }
            Aggregate::UnanimousPass | Aggregate::ReviewPanel => Ok(()),
        }
    }
}

/// Why a review-panel member's word is refused before the join. Its
/// text is the poisoned aggregate's evidence.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub(crate) enum MemberRefusal {
    #[error("the member has no compiled result set")]
    Unbound,
    #[error("member result '{word}' is not among its declared results [{}]", .results.join(", "))]
    Undeclared { word: String, results: Vec<String> },
    #[error(transparent)]
    Word(#[from] WordRefusal),
}

/// A panel's join: its declared aggregate and, by member, the law each
/// member's word answers to.
#[derive(Debug, Clone)]
pub(crate) struct PanelJoin {
    aggregate: Aggregate,
    laws: MemberLaws,
}

/// Each member's compiled law, by its bare name.
pub(crate) type MemberLaws = BTreeMap<String, MemberLaw>;

/// The laws of `members`, by name.
pub(crate) fn laws_of(members: &[PanelMember]) -> MemberLaws {
    (members.iter())
        .map(|member| (member.name.clone(), member.law.clone()))
        .collect()
}

impl PanelJoin {
    /// The join of `members` under `aggregate`, each with its own law.
    pub(crate) fn of(aggregate: Aggregate, members: &[PanelMember]) -> Self {
        let laws = laws_of(members);
        PanelJoin { aggregate, laws }
    }

    /// The join over a partial retry's members: the fresh members' laws,
    /// and every carried member's from the declared panel's `laws`.
    pub(crate) fn carrying(&self, laws: &MemberLaws) -> Self {
        let mut joined = laws.clone();
        joined.extend(self.laws.clone());
        PanelJoin {
            aggregate: self.aggregate,
            laws: joined,
        }
    }

    /// The join of the members `names` under `aggregate`, each under
    /// [`MemberLaw::open`].
    #[cfg(test)]
    pub(crate) fn open(aggregate: Aggregate, names: &[&str]) -> Self {
        Self::under(
            aggregate,
            names.iter().map(|name| (*name, MemberLaw::open())),
        )
    }

    /// The join under `aggregate` of the members `laws` names, each
    /// under its own law.
    #[cfg(test)]
    pub(crate) fn under<'n>(
        aggregate: Aggregate,
        laws: impl IntoIterator<Item = (&'n str, MemberLaw)>,
    ) -> Self {
        let laws = (laws.into_iter())
            .map(|(name, law)| (name.to_string(), law))
            .collect();
        PanelJoin { aggregate, laws }
    }

    /// Refuse `word` from the member `name` when its law does not admit
    /// it, or when it is not bound to the member's `findings` and the
    /// residual inputs of its `payload` against the owning seat's pair
    /// (decision 0109 ruling 2), whatever the member's class. Only a
    /// `review-panel` member is held by its law: a `unanimous-pass` member
    /// keeps that aggregate's own check.
    fn refusal(
        &self,
        (name, word): (&str, &str),
        findings: &[Finding],
        payload: &Value,
    ) -> Option<MemberRefusal> {
        if self.aggregate != Aggregate::ReviewPanel {
            return None;
        }
        let Some(law) = self.laws.get(name) else {
            return Some(MemberRefusal::Unbound);
        };
        if !law.results.iter().any(|lawful| lawful == word) {
            return Some(MemberRefusal::Undeclared {
                word: word.to_string(),
                results: law.results.clone(),
            });
        }
        let inputs = payload.get("inputs").and_then(Value::as_object);
        bind(word, findings, inputs, law.residual)
            .err()
            .map(MemberRefusal::from)
    }

    /// Deterministic, order-independent aggregation of member results. A
    /// member payload without a readable result string, with findings
    /// that are not the engine's stamped records, or, in a review panel,
    /// with a word its compiled law refuses, poisons the aggregate into a
    /// marker the engine's own schema validation parks with the full
    /// member evidence attached — never coerced (law 0001); a refused
    /// member's reason rides beside it under `refused`. A member result
    /// outside the three ranked words ranks WORST and is returned
    /// verbatim: declared by the step, it is a word the step can emit;
    /// undeclared, a non-final step's `vocabulary_problem` fails the
    /// attempt, and only a final step's word reaches decide(), whose
    /// declared-results check parks it with evidence.
    pub(crate) fn aggregate_results(&self, members: &[MemberVerdict]) -> Value {
        let read = match Verdicts::read(members, self) {
            Ok(read) => read,
            Err(refused) => return poisoned(members, refused),
        };
        let meta = json!({"members": read.notes, "verdicts": read.verdicts});
        let mut result = match self.aggregate {
            Aggregate::UnanimousPass => unanimous_pass(&read.parsed, meta),
            Aggregate::ReviewPanel => review_panel(&read.parsed, meta),
        };
        if !read.findings.is_empty() {
            brokkr_core::findings::journal(&mut result, &read.findings);
        }
        result
    }
}

/// The aggregate `members` poison: the marker, every member's evidence,
/// the refused member's reason where one was refused by its law, and the
/// members' admitted findings kept keyed.
fn poisoned(members: &[MemberVerdict], refused: Option<(String, MemberRefusal)>) -> Value {
    let evidence: Map<String, Value> = members.iter().map(MemberVerdict::evidence).collect();
    let mut poisoned = json!({
        "result": Aggregate::MEMBER_SCHEMA_INVALID,
        "notes": {"members": Value::Object(evidence)},
    });
    if let Some((name, refusal)) = refused {
        poisoned["notes"]["refused"] = json!({ name: refusal.to_string() });
    }
    // The members' admitted findings stay the record's keyed findings,
    // each under its member's own site, so a refused aggregate hides none
    // in its notes (fix round 0109a1r6, W4).
    let admitted = MemberVerdict::admitted(members);
    if !admitted.is_empty() {
        brokkr_core::findings::journal(&mut poisoned, &admitted);
    }
    poisoned
}

/// A panel member's findings as the aggregate joins them, typed: those
/// admission stamped for a fresh member, or a carried member's read once
/// from its journaled result; or a carried record whose findings do not
/// read, which poisons the aggregate.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum MemberFindings {
    Read(Vec<Finding>),
    Unread,
}

impl MemberFindings {
    /// The findings a journaled `result` records, taken out of it: none
    /// where it records none; [`MemberFindings::Unread`], leaving them in
    /// place as evidence, where they do not read as stamped records.
    pub(crate) fn taken_from(result: &mut Value) -> Self {
        let Some(recorded) = result.get("findings") else {
            return MemberFindings::Read(Vec::new());
        };
        let Ok(findings) = Vec::<Finding>::deserialize(recorded) else {
            return MemberFindings::Unread;
        };
        let _taken = result
            .as_object_mut()
            .and_then(|fields| fields.remove("findings"));
        MemberFindings::Read(findings)
    }
}

/// One panel member's verdict as the aggregate reads it: its name, its
/// result's payload, and its findings kept typed beside it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MemberVerdict {
    pub(crate) name: String,
    pub(crate) payload: Value,
    pub(crate) findings: MemberFindings,
}

impl MemberVerdict {
    /// A member verdict read from its journaled `payload`, as a carried
    /// member's is: the tests' one builder.
    #[cfg(test)]
    pub(crate) fn journaled(name: &str, mut payload: Value) -> Self {
        let findings = MemberFindings::taken_from(&mut payload);
        MemberVerdict {
            name: name.to_string(),
            payload,
            findings,
        }
    }

    /// Every `(name, payload)` pair read as [`MemberVerdict::journaled`].
    #[cfg(test)]
    pub(crate) fn all(pairs: &[(&str, Value)]) -> Vec<Self> {
        (pairs.iter())
            .map(|(name, payload)| Self::journaled(name, payload.clone()))
            .collect()
    }

    /// The member's payload as a poisoned aggregate's evidence names it.
    /// The findings it read are the record's own keyed findings, so they
    /// are not written here a second time (fix round 0109a1r6, W4); a
    /// payload whose findings do not read keeps them in place.
    fn evidence(&self) -> (String, Value) {
        (self.name.clone(), self.payload.clone())
    }

    /// Every finding `members` read, in declared order: what a poisoned
    /// aggregate keeps as its keyed findings, each with its member's site.
    fn admitted(members: &[MemberVerdict]) -> Vec<Finding> {
        (members.iter())
            .filter_map(|member| match &member.findings {
                MemberFindings::Read(findings) => Some(findings.iter().cloned()),
                MemberFindings::Unread => None,
            })
            .flatten()
            .collect()
    }
}

/// Every member's result string, read once for the aggregate: by member,
/// its notes and its verdict as evidence, and beside its name the result
/// and the payload it came in; and every member's findings, typed, in
/// declared order, each as the engine stamped it with that member's own
/// site (decision 0109 ruling 3), so a copy of a sibling's finding is
/// another key and never a borrowed one.
struct Verdicts<'a> {
    notes: Map<String, Value>,
    verdicts: Map<String, Value>,
    parsed: Vec<(&'a str, &'a str, &'a Value)>,
    findings: Vec<Finding>,
}

impl<'a> Verdicts<'a> {
    /// The members' verdicts, or the poison when any member's payload has
    /// no readable result string, carries findings that do not read as
    /// the engine's stamped records, or, under `join`, writes a word its
    /// law refuses, which the poison then names.
    fn read(
        members: &'a [MemberVerdict],
        join: &PanelJoin,
    ) -> Result<Self, Option<(String, MemberRefusal)>> {
        let mut notes = Map::new();
        let mut verdicts = Map::new();
        let mut parsed: Vec<(&str, &str, &Value)> = Vec::new();
        let mut findings = Vec::new();
        for MemberVerdict {
            name,
            payload,
            findings: reported,
        } in members
        {
            if let Some(note) = payload.get("notes") {
                notes.insert(name.clone(), note.clone());
            }
            let result = payload.get("result").and_then(Value::as_str).ok_or(None)?;
            verdicts.insert(name.clone(), Value::String(result.to_string()));
            parsed.push((name.as_str(), result, payload));
            let MemberFindings::Read(reported) = reported else {
                return Err(None);
            };
            if let Some(refusal) = join.refusal((name, result), reported, payload) {
                return Err(Some((name.clone(), refusal)));
            }
            findings.extend(reported.iter().cloned());
        }
        Ok(Verdicts {
            notes,
            verdicts,
            parsed,
            findings,
        })
    }
}

/// [`Aggregate::UnanimousPass`]: "pass" only when every member reports
/// "pass", otherwise "fail".
fn unanimous_pass(parsed: &[(&str, &str, &Value)], meta: Value) -> Value {
    // Only the declared vocabulary aggregates; anything else is
    // never coerced to "fail" (law 0001) — it poisons the result
    // so decide() parks with the member evidence.
    if parsed.iter().any(|(_, r, _)| *r != "pass" && *r != "fail") {
        return json!({
            "result": Aggregate::MEMBER_SCHEMA_INVALID,
            "notes": meta,
        });
    }
    let all_pass = parsed.iter().all(|(_, r, _)| *r == "pass");
    json!({
        "result": if all_pass { "pass" } else { "fail" },
        "notes": meta,
    })
}

/// [`Aggregate::ReviewPanel`]: the worst member's result wins, over
/// clean < residual < security-hold, with the members' residual inputs
/// joined beside it.
fn review_panel(parsed: &[(&str, &str, &Value)], meta: Value) -> Value {
    let rank = |r: &str| match r {
        "clean" => 0,
        "residual" => 1,
        "security-hold" => 2,
        _ => 3, // unknown ranks worst: fail closed via decide()
    };
    let worst = parsed
        .iter()
        .max_by_key(|(_, r, _)| rank(r))
        .expect("panels have members");
    let joined = ReviewInputs::join(parsed);
    let mut inputs = Map::new();
    // Backward-compatible evidence for frozen/third-party tables:
    // judges never fix, so the only lawful value is false. Shipped
    // tables no longer declare or read it and therefore drop it.
    inputs.insert("fixes_applied".into(), Value::Bool(false));
    inputs.insert("spec_defect".into(), Value::Bool(joined.spec_defect));
    // Both residual inputs on every result, `none` and `false` when no
    // member reports one, so an all-clean panel's word is bound to a
    // declared pair rather than an absent one (decision 0109 ruling 2).
    inputs.insert(
        "max_residual_severity".into(),
        Value::String(SEVERITY_ORDER[joined.severity_rank].to_string()),
    );
    inputs.insert(
        "has_security_residual".into(),
        Value::Bool(joined.has_security),
    );
    json!({"result": worst.1, "inputs": inputs, "notes": meta})
}

/// What a review panel's members report in their own `inputs`, joined:
/// the highest residual severity any names (one outside
/// [`SEVERITY_ORDER`] counts as none), and whether any names a security
/// residual or a spec defect.
struct ReviewInputs {
    severity_rank: usize,
    has_security: bool,
    spec_defect: bool,
}

impl ReviewInputs {
    fn join(parsed: &[(&str, &str, &Value)]) -> Self {
        let mut severity_rank = 0usize;
        let mut has_security = false;
        let mut spec_defect = false;
        for (_, _, payload) in parsed {
            let Some(inputs) = payload.get("inputs").and_then(Value::as_object) else {
                continue;
            };
            if let Some(s) = inputs.get("max_residual_severity").and_then(Value::as_str) {
                if let Some(i) = SEVERITY_ORDER.iter().position(|x| *x == s) {
                    severity_rank = severity_rank.max(i);
                }
            }
            has_security |= inputs
                .get("has_security_residual")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            spec_defect |= inputs
                .get("spec_defect")
                .and_then(Value::as_bool)
                .unwrap_or(false);
        }
        ReviewInputs {
            severity_rank,
            has_security,
            spec_defect,
        }
    }
}

#[cfg(test)]
mod tests;
