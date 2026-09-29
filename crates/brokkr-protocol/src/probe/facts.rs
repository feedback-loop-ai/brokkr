//! The probe report's typed shape: every fact the probe writes, and the
//! three readings a fact may have. The report is read back only to find
//! drift, so it is parsed once, strictly, into these same types.

use serde::{Deserialize, Serialize};

use super::ProbeError;

/// The report format this probe writes and reads back.
pub(crate) const PROBE_VERSION: &str = "brokkr.harness-probe/v1";

/// One fact, in exactly one of its three readings. `Unmeasured` is not a
/// guess at either of the other two: it says why the probe could not
/// look, and the eligibility verdict reads it as not established.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase", deny_unknown_fields)]
pub(crate) enum Fact<T> {
    /// The CLI showed it, and `evidence` says where.
    Measured { value: T, evidence: String },
    /// The probe could not look; `why` says what stood in the way.
    Unmeasured { why: String },
    /// The CLI refused the mechanism the fact is about.
    Unsupported { evidence: String },
}

impl<T: Serialize> Fact<T> {
    pub(crate) fn measured(value: T, evidence: impl Into<String>) -> Fact<T> {
        Fact::Measured {
            value,
            evidence: evidence.into(),
        }
    }

    pub(crate) fn unmeasured(why: impl Into<String>) -> Fact<T> {
        Fact::Unmeasured { why: why.into() }
    }

    pub(crate) fn value(&self) -> Option<&T> {
        match self {
            Fact::Measured { value, .. } => Some(value),
            Fact::Unmeasured { .. } | Fact::Unsupported { .. } => None,
        }
    }

    /// The reading drift compares: the status and the measured value,
    /// never the evidence, whose wording may change between two runs
    /// that measured the same thing.
    pub(crate) fn reading(&self) -> String {
        match self {
            Fact::Measured { value, .. } => {
                format!(
                    "measured {}",
                    serde_json::to_string(value).unwrap_or_default()
                )
            }
            Fact::Unmeasured { .. } => "unmeasured".to_string(),
            Fact::Unsupported { .. } => "unsupported".to_string(),
        }
    }
}

/// The headless launch that ran one turn, with the CLI's own paths and
/// prompt left as `{cli}`, `{workdir}` and `{prompt}`, and how it exited.
/// `exit` is `null` when no exit code was reported: a signal, or the
/// probe's deadline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Headless {
    pub(crate) argv: Vec<String>,
    pub(crate) exit: Option<i32>,
}

/// Where the turn's events were read from and what they were.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Events {
    /// `stdout`, or the transcript path the events were read from when
    /// stdout carried none.
    pub(crate) source: String,
    /// Every line was one JSON object, save `non_json_lines` of them.
    pub(crate) format: String,
    pub(crate) non_json_lines: usize,
    /// Each event's `type` (and `/subtype`), in first-seen order.
    pub(crate) types: Vec<String>,
}

/// Which event announced the session, and under which key.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Session {
    pub(crate) event: String,
    pub(crate) key: String,
}

/// Whether a usage count stands for its event or repeats its message's
/// (#402: one message's usage, restated on each of its stream events,
/// counted once per event, doubles the total).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Counting {
    PerEvent,
    RepeatedPerMessage,
}

/// Where usage is reported, which counters it names, and how to count it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Usage {
    /// `<event type> <JSON pointer>` for each place a usage object sat.
    pub(crate) locations: Vec<String>,
    pub(crate) counters: Vec<String>,
    pub(crate) counting: Counting,
}

/// How one refusal looked: its exit status and the line that said it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Refusal {
    pub(crate) exit: Option<i32>,
    pub(crate) excerpt: String,
}

/// The refusal classes a seat failure is classified into.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Refusals {
    pub(crate) auth: Fact<Refusal>,
    pub(crate) config: Fact<Refusal>,
    pub(crate) rate_limit: Fact<Refusal>,
    pub(crate) outage: Fact<Refusal>,
}

/// Everything the probe measured of the harness.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Facts {
    pub(crate) headless: Fact<Headless>,
    pub(crate) events: Fact<Events>,
    pub(crate) session: Fact<Session>,
    pub(crate) usage: Fact<Usage>,
    /// `<event type> <JSON pointer>` for each place a cost sat; empty when
    /// the CLI reports none.
    pub(crate) cost: Fact<Vec<String>>,
    pub(crate) refusals: Refusals,
    pub(crate) efforts: Fact<Vec<String>>,
    /// The CLI's own tools in a plain turn (MCP tools left out).
    pub(crate) tools: Fact<Vec<String>>,
    /// The CLI's own tools left under the adapter's hands argv: emptied
    /// when this is measured empty.
    pub(crate) boxed_tools: Fact<Vec<String>>,
    /// The status the CLI reported for the hands MCP server it was given.
    pub(crate) mcp_server: Fact<String>,
    pub(crate) native_egress: Fact<Vec<String>>,
    /// True when no native egress tool is left under the hands argv.
    pub(crate) egress_off: Fact<bool>,
    /// True when a turn ran under a scratch HOME with only the bound
    /// credentials (#467).
    pub(crate) config_isolation: Fact<bool>,
    /// Whether a user-scope MCP server planted in the scratch HOME
    /// reached the plain turn, and the boxed one (#467).
    pub(crate) user_mcp_unboxed: Fact<bool>,
    pub(crate) user_mcp_boxed: Fact<bool>,
    /// The transcript files a turn wrote under HOME, `~`-relative, with
    /// the session id as `{session}` and every digit run as `{n}`.
    pub(crate) transcripts: Fact<Vec<String>>,
    pub(crate) resume: Fact<String>,
}

impl Facts {
    /// Every fact's reading by name, in report order: what drift compares.
    pub(crate) fn readings(&self) -> Vec<(&'static str, String)> {
        let refusals = &self.refusals;
        vec![
            ("headless", self.headless.reading()),
            ("events", self.events.reading()),
            ("session", self.session.reading()),
            ("usage", self.usage.reading()),
            ("cost", self.cost.reading()),
            ("refusals.auth", refusals.auth.reading()),
            ("refusals.config", refusals.config.reading()),
            ("refusals.rate_limit", refusals.rate_limit.reading()),
            ("refusals.outage", refusals.outage.reading()),
            ("efforts", self.efforts.reading()),
            ("tools", self.tools.reading()),
            ("boxed_tools", self.boxed_tools.reading()),
            ("mcp_server", self.mcp_server.reading()),
            ("native_egress", self.native_egress.reading()),
            ("egress_off", self.egress_off.reading()),
            ("config_isolation", self.config_isolation.reading()),
            ("user_mcp_unboxed", self.user_mcp_unboxed.reading()),
            ("user_mcp_boxed", self.user_mcp_boxed.reading()),
            ("transcripts", self.transcripts.reading()),
            ("resume", self.resume.reading()),
        ]
    }
}

/// Decision 0075 ruling 4's seat eligibility, derived from the facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Verdict {
    Boxed,
    UnboxedOnly,
    ToolLessOnly,
    Refused,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Eligibility {
    pub(crate) verdict: Verdict,
    pub(crate) reason: String,
}

/// Whether the adapter's declared field and the one the facts imply agree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Agreement {
    Agrees,
    Differs,
    /// The fact the field rests on is not measured, so nothing is compared.
    NotCompared,
}

/// One adapter field beside the value the probe's facts imply for it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FieldRow {
    pub(crate) field: String,
    pub(crate) declared: String,
    pub(crate) implied: String,
    pub(crate) agreement: Agreement,
}

/// One reading that moved since the report this run replaced.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DriftRow {
    pub(crate) fact: String,
    pub(crate) before: String,
    pub(crate) after: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Cli {
    /// The command the probe launched, as the operator named it.
    pub(crate) command: String,
    /// The first line `--version` printed.
    pub(crate) version: Fact<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Host {
    pub(crate) os: String,
    pub(crate) arch: String,
}

/// The probe's whole output: one CLI, measured once, on one host and day.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub(crate) probe: String,
    pub(crate) adapter: String,
    pub(crate) cli: Cli,
    pub(crate) host: Host,
    pub(crate) date: String,
    pub(crate) facts: Facts,
    pub(crate) adapter_fields: Vec<FieldRow>,
    pub(crate) eligibility: Eligibility,
    pub(crate) drift: Vec<DriftRow>,
}

impl Report {
    /// Read a report this probe wrote, refusing anything else: a report
    /// of another format version, or not a report at all, is never
    /// compared against, because drift from a misread report is noise.
    pub fn parse(text: &str) -> Result<Report, ProbeError> {
        let report: Report = serde_json::from_str(text).map_err(|error| ProbeError::Report {
            problem: error.to_string(),
        })?;
        if report.probe != PROBE_VERSION {
            return Err(ProbeError::Report {
                problem: format!("its format is '{}'", report.probe),
            });
        }
        Ok(report)
    }

    /// This report, with every reading that moved since `previous`.
    pub fn with_drift_from(mut self, previous: &Report) -> Report {
        self.drift = super::judge::drift(previous, &self);
        self
    }

    /// One line per drifted reading, for the operator's terminal.
    pub fn drift_lines(&self) -> Vec<String> {
        self.drift
            .iter()
            .map(|row| format!("drift: {}: {} -> {}", row.fact, row.before, row.after))
            .collect()
    }
}
