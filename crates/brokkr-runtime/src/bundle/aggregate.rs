//! A panel's aggregate (#349), in one place: the word a bundle writes for
//! it, its parse and closed vocabulary, the results it can emit, and the
//! aggregation the engine runs over the members' results.

use brokkr_core::policy::SEVERITY_ORDER;
use serde_json::{json, Map, Value};

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

    pub(super) fn required_results(&self) -> &'static [&'static str] {
        match self {
            Aggregate::UnanimousPass => &["pass", "fail"],
            Aggregate::ReviewPanel => &["clean", "residual", "security-hold"],
        }
    }

    /// Deterministic, order-independent aggregation of member results. A
    /// member payload without a readable result string poisons the
    /// aggregate into a marker the engine's own schema validation parks
    /// with the full member evidence attached — never coerced (law 0001).
    /// A member result outside the vocabulary ranks WORST and flows to
    /// decide(), whose declared-results check parks it with evidence.
    pub(crate) fn aggregate_results(self, members: &[(String, Value)]) -> Value {
        let Some(read) = Verdicts::read(members) else {
            let evidence: Map<String, Value> = members.iter().cloned().collect();
            return json!({
                "result": "__member-schema-invalid__",
                "notes": {"members": Value::Object(evidence)},
            });
        };
        let meta = json!({"members": read.notes, "verdicts": read.verdicts});
        match self {
            Aggregate::UnanimousPass => unanimous_pass(&read.parsed, meta),
            Aggregate::ReviewPanel => review_panel(&read.parsed, meta),
        }
    }
}

/// Every member's result string, read once for the aggregate: by member,
/// its notes and its verdict as evidence, and beside its name the result
/// and the payload it came in.
struct Verdicts<'a> {
    notes: Map<String, Value>,
    verdicts: Map<String, Value>,
    parsed: Vec<(&'a str, &'a str, &'a Value)>,
}

impl<'a> Verdicts<'a> {
    /// The members' verdicts, or `None` when any member's payload has no
    /// readable result string.
    fn read(members: &'a [(String, Value)]) -> Option<Self> {
        let mut notes = Map::new();
        let mut verdicts = Map::new();
        let mut parsed: Vec<(&str, &str, &Value)> = Vec::new();
        for (name, payload) in members {
            if let Some(note) = payload.get("notes") {
                notes.insert(name.clone(), note.clone());
            }
            let result = payload.get("result").and_then(Value::as_str)?;
            verdicts.insert(name.clone(), Value::String(result.to_string()));
            parsed.push((name.as_str(), result, payload));
        }
        Some(Verdicts {
            notes,
            verdicts,
            parsed,
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
            "result": "__member-schema-invalid__",
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
    if worst.1 == "residual" || joined.severity_rank > 0 {
        inputs.insert(
            "max_residual_severity".into(),
            Value::String(SEVERITY_ORDER[joined.severity_rank].to_string()),
        );
        inputs.insert(
            "has_security_residual".into(),
            Value::Bool(joined.has_security),
        );
    }
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
