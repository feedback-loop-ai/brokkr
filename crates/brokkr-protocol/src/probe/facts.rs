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

    /// What the fact says for itself: its evidence, or why it is unmeasured.
    pub(crate) fn account(&self) -> &str {
        match self {
            Fact::Measured { evidence, .. } | Fact::Unsupported { evidence } => evidence,
            Fact::Unmeasured { why } => why,
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

/// How a usage count is counted (#402: one message's usage, restated on
/// each of its stream events, counted once per event, doubles the total).
/// A stream shows only the repetition; nothing a turn prints tells a
/// per-event count from a message seen once, so that stays unmeasured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Counting {
    RepeatedPerMessage,
}

/// Where usage is reported, which counters it names, and how to count it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Usage {
    /// `<event type> <JSON pointer>` for each place a usage object sat.
    pub(crate) locations: Vec<String>,
    pub(crate) counters: Vec<String>,
    /// Unmeasured unless one message carried the same usage on several
    /// events; usage that names no message is listed in its account.
    pub(crate) counting: Fact<Counting>,
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

/// One native capability the adapter declares, as a realm grants it, the
/// tools it maps it to, and what switches it off: the declared OFF
/// controls when the turn under them listed none of those tools, or
/// `unsupported` with what the probe saw (decision 0065 ruling 4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Capability {
    pub(crate) capability: String,
    pub(crate) tools: Vec<String>,
    pub(crate) off: Fact<Vec<String>>,
}

/// Everything the probe measured of the harness.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Facts {
    pub(crate) headless: Fact<Headless>,
    pub(crate) events: Fact<Events>,
    pub(crate) session: Fact<Session>,
    pub(crate) usage: Fact<Usage>,
    /// `<event type> <JSON pointer>` for each place a cost sat; unmeasured
    /// when no event carries a key the probe knows a cost by.
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
    /// The plain turn's egress tools; unmeasured when a tool is named that
    /// the probe knows neither as egress nor as local.
    pub(crate) native_egress: Fact<Vec<String>>,
    /// True when no native egress tool is left under the declared OFF
    /// controls.
    pub(crate) egress_off: Fact<bool>,
    /// Each native capability the adapter declares, with what switches it
    /// off; unmeasured when the plain turn lists a tool none of them maps.
    pub(crate) capabilities: Fact<Vec<Capability>>,
    /// True when no MCP server other than the hands server, the one
    /// planted in the scratch HOME's user-scope configuration included,
    /// reached the boxed turn, or, with no boxed turn read, the plain one
    /// (#467).
    pub(crate) config_isolation: Fact<bool>,
    /// Whether an MCP server other than the hands server reached the plain
    /// turn, and the boxed one: named in its server listing, or by a
    /// listed `mcp__<server>__` tool (#467).
    pub(crate) user_mcp_unboxed: Fact<bool>,
    pub(crate) user_mcp_boxed: Fact<bool>,
    /// The same of the turn under the declared OFF controls, the launch a
    /// seat outside the box granted nothing makes (#484).
    pub(crate) user_mcp_off: Fact<bool>,
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
            ("capabilities", self.capability_readings().reading()),
            ("config_isolation", self.config_isolation.reading()),
            ("user_mcp_unboxed", self.user_mcp_unboxed.reading()),
            ("user_mcp_boxed", self.user_mcp_boxed.reading()),
            ("user_mcp_off", self.user_mcp_off.reading()),
            ("transcripts", self.transcripts.reading()),
            ("resume", self.resume.reading()),
        ]
    }

    /// Each capability's off switch by its reading, its evidence left out.
    fn capability_readings(&self) -> Fact<Vec<(String, String)>> {
        self.capabilities.clone().map(|capabilities| {
            capabilities
                .into_iter()
                .map(|capability| (capability.capability, capability.off.reading()))
                .collect()
        })
    }
}

/// Decision 0075 ruling 4's seat eligibility, derived from the facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Verdict {
    /// Boxed offices, its plain turn also shown to keep the planted
    /// user-scope server out.
    Boxed,
    /// Boxed offices only: the box keeps the planted user-scope server
    /// out, and its plain turn, which an office outside the box launches,
    /// is not shown to (#467). Operator ruling B, 2026-09-29.
    BoxedOnly,
    UnboxedOnly,
    /// Seated only in a realm that grants each native capability the
    /// probe measured no off switch for, which the reason names. A
    /// tool-less office switches nothing off inside the CLI, so it admits
    /// nothing more (operator ruling A, 2026-09-29: proposed decision
    /// 0075 ruling 4 read with decision 0065 ruling 4).
    GrantingRealmsOnly,
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
