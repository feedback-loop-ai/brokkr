//! A verdict's word, bound to its findings and to its declared residual
//! pair (decision 0109 ruling 2). Either body of evidence can refuse a
//! passing word; neither rewrites it. [`bind`] is the one check: the
//! engine runs it on a gate's result at decide, on a gate step's result
//! before the step settles, and on every review-panel member's result
//! before its word is joined. A refusal is a typed fact the engine
//! journals as a malformed result; nothing here derives a word.

use serde::Deserialize;
use serde_json::{Map, Value};
use thiserror::Error;

use super::Finding;
use crate::policy::Severity;

/// The closed set of non-passing words. It is engine data, never a
/// reading of a phase table, so no recipe `when` can drop the check; a
/// word it does not name, an operator's own included, is passing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NonPassing {
    Residual,
    Drift,
    Ambiguous,
    CleanUnverified,
    Fail,
    SecurityHold,
    Upstream,
    Escalate,
}

impl NonPassing {
    const ALL: [NonPassing; 8] = [
        NonPassing::Residual,
        NonPassing::Drift,
        NonPassing::Ambiguous,
        NonPassing::CleanUnverified,
        NonPassing::Fail,
        NonPassing::SecurityHold,
        NonPassing::Upstream,
        NonPassing::Escalate,
    ];

    /// The word a table and a seat write for it.
    pub(crate) const fn word(self) -> &'static str {
        match self {
            NonPassing::Residual => "residual",
            NonPassing::Drift => "drift",
            NonPassing::Ambiguous => "ambiguous",
            NonPassing::CleanUnverified => "clean-unverified",
            NonPassing::Fail => "fail",
            NonPassing::SecurityHold => "security-hold",
            NonPassing::Upstream => "upstream",
            NonPassing::Escalate => "escalate",
        }
    }

    /// The non-passing word `word` is, or `None` for a passing word.
    pub(crate) fn named(word: &str) -> Option<NonPassing> {
        NonPassing::ALL
            .into_iter()
            .find(|non_passing| non_passing.word() == word)
    }
}

/// One input of the residual pair, the two aggregate inputs ruling 2
/// holds a passing word to. A seat's other inputs (`spec_defect`,
/// `fixes_applied`, `change`) are never this check's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResidualInput {
    MaxResidualSeverity,
    HasSecurityResidual,
}

impl ResidualInput {
    const ALL: [ResidualInput; 2] = [
        ResidualInput::MaxResidualSeverity,
        ResidualInput::HasSecurityResidual,
    ];

    /// The evaluator input it is.
    pub const fn name(self) -> &'static str {
        match self {
            ResidualInput::MaxResidualSeverity => "max_residual_severity",
            ResidualInput::HasSecurityResidual => "has_security_residual",
        }
    }
}

impl std::fmt::Display for ResidualInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// Which of the residual pair a seat's compiled input set includes: a
/// seat that declares both must write both, one that declares one must
/// write that one, and one that declares neither is not held by the
/// aggregate arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResidualPair {
    severity: bool,
    security: bool,
}

impl ResidualPair {
    /// The pair a compiled input set `inputs` declares.
    pub fn declared_in<S: AsRef<str>>(inputs: &[S]) -> ResidualPair {
        let declares =
            |input: ResidualInput| inputs.iter().any(|name| name.as_ref() == input.name());
        ResidualPair {
            severity: declares(ResidualInput::MaxResidualSeverity),
            security: declares(ResidualInput::HasSecurityResidual),
        }
    }

    fn includes(self, input: ResidualInput) -> bool {
        match input {
            ResidualInput::MaxResidualSeverity => self.severity,
            ResidualInput::HasSecurityResidual => self.security,
        }
    }
}

/// Why a word is refused. The word echoed is one the seat's or the
/// member's declared result set already admitted; no variant echoes a
/// value outside a closed vocabulary.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum WordRefusal {
    #[error(
        "the word '{word}' stands over a security finding, which requires 'security-hold' \
         (decision 0109 ruling 2)"
    )]
    SecurityFinding { word: String },
    #[error(
        "the passing word '{word}' stands over a blocking finding (severity above low); a \
         blocking finding admits 'residual', 'clean-unverified' or 'security-hold' \
         (decision 0109 ruling 2)"
    )]
    BlockingFinding { word: String },
    #[error(
        "the passing word '{word}' stands over a declared max_residual_severity of '{}' \
         (decision 0109 ruling 2)",
        .severity.name()
    )]
    ResidualSeverity { word: String, severity: Severity },
    #[error(
        "the passing word '{word}' stands over a declared has_security_residual of true \
         (decision 0109 ruling 2)"
    )]
    SecurityResidual { word: String },
    #[error(
        "the passing word '{word}' omits '{input}', which the seat declares (decision 0109 \
         ruling 2)"
    )]
    Absent { word: String, input: ResidualInput },
    #[error(
        "the passing word '{word}' carries '{input}' as null or as no value of its vocabulary \
         (decision 0109 ruling 2)"
    )]
    Unreadable { word: String, input: ResidualInput },
    #[error("the result's findings are not the engine's stamped records")]
    Findings,
}

/// Whether `finding` blocks: a severity above `low`, or a security flag.
fn blocking(finding: &Finding) -> bool {
    finding.severity() > Severity::Low || finding.security()
}

/// What a declared residual input reads as: clear, standing, or neither.
enum Declared {
    Clear,
    Severity(Severity),
    Security,
    Unreadable,
}

impl Declared {
    fn read(input: ResidualInput, value: &Value) -> Declared {
        match input {
            ResidualInput::MaxResidualSeverity => match value.as_str().and_then(Severity::named) {
                Some(Severity::None) => Declared::Clear,
                Some(severity) => Declared::Severity(severity),
                None => Declared::Unreadable,
            },
            ResidualInput::HasSecurityResidual => match value.as_bool() {
                Some(false) => Declared::Clear,
                Some(true) => Declared::Security,
                None => Declared::Unreadable,
            },
        }
    }
}

/// The aggregate arm: each residual input `pair` declares is present,
/// readable and clear under a passing `word`.
fn aggregate(
    word: &str,
    inputs: Option<&Map<String, Value>>,
    pair: ResidualPair,
) -> Result<(), WordRefusal> {
    let word = || word.to_string();
    for input in ResidualInput::ALL
        .into_iter()
        .filter(|input| pair.includes(*input))
    {
        let Some(value) = inputs.and_then(|inputs| inputs.get(input.name())) else {
            return Err(WordRefusal::Absent {
                word: word(),
                input,
            });
        };
        match Declared::read(input, value) {
            Declared::Clear => {}
            Declared::Severity(severity) => {
                return Err(WordRefusal::ResidualSeverity {
                    word: word(),
                    severity,
                })
            }
            Declared::Security => return Err(WordRefusal::SecurityResidual { word: word() }),
            Declared::Unreadable => {
                return Err(WordRefusal::Unreadable {
                    word: word(),
                    input,
                })
            }
        }
    }
    Ok(())
}

/// Bind `word` to its `findings` and to the residual `inputs` it was
/// written with, against the `pair` its seat compiles (decision 0109
/// ruling 2). A security finding requires `security-hold`; any other
/// blocking finding refuses a passing word; a passing word is refused
/// over a standing, absent or unreadable declared residual input. A
/// non-passing word other than the hold is never this check's to refuse
/// except over a security finding.
pub fn bind(
    word: &str,
    findings: &[Finding],
    inputs: Option<&Map<String, Value>>,
    pair: ResidualPair,
) -> Result<(), WordRefusal> {
    let held = word == NonPassing::SecurityHold.word();
    if !held && findings.iter().any(Finding::security) {
        return Err(WordRefusal::SecurityFinding {
            word: word.to_string(),
        });
    }
    if NonPassing::named(word).is_some() {
        return Ok(());
    }
    if findings.iter().any(blocking) {
        return Err(WordRefusal::BlockingFinding {
            word: word.to_string(),
        });
    }
    aggregate(word, inputs, pair)
}

/// [`bind`] over a journaled `result` whose word is `word`: its stamped
/// `findings` (absent reading as none) and its `inputs`. Every other
/// field, an engine-carried `follow_ups` among them, leaves the word as
/// it is.
pub fn bind_journaled(
    result: &Map<String, Value>,
    word: &str,
    pair: ResidualPair,
) -> Result<(), WordRefusal> {
    let findings = match result.get("findings") {
        None => Vec::new(),
        Some(findings) => {
            Vec::<Finding>::deserialize(findings).map_err(|_| WordRefusal::Findings)?
        }
    };
    let inputs = result.get("inputs").and_then(Value::as_object);
    bind(word, &findings, inputs, pair)
}

#[cfg(test)]
mod tests;
