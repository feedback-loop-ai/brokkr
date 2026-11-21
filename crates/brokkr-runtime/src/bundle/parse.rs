//! The site parsers (#349, ask 6): a seat's body, a select and its cases,
//! a panel and its members, each a method of the one [`SiteCompiler`]
//! that holds what every site of a compile shares. The sequence parser
//! and the agent resolver take the compiler from `bundle.rs`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use brokkr_core::findings::ResidualPair;
use brokkr_core::realms::Boundary;
use brokkr_protocol::hands::HandsSpec;
use serde_json::{Map, Value};

use super::site_form::{classify, CaseShape, MemberShape, SeatShape, CASE, MEMBER, SEAT};
use super::{
    built_in_model_driver, command_parts, enforce_model_policy, parse_class, parse_command,
    parse_role, parse_sequence, record_hands, record_inline_tools, refuse_class_without_a_driver,
    resolve_reference, Aggregate, Candidate, Charters, CompileError, Dialect, Invocation,
    MemberLaw, Opened, PanelMember, ResolvedSeat, SeatBody, SeatClass, SiteFacts, SiteLaw,
    SELECT_STRATEGIES,
};
use crate::argv::Executable;

/// One compile's site parser. It owns the compile's two accumulators —
/// what the sites opened, which the resolver reads, and the canonical site
/// family every fact is written into — and borrows what every site is
/// judged against: the boundary, the engine executable a `{brokkr}` expands
/// to, the realm's dialect, the charters bound so far, and the layer roots
/// with each select case's origin among them.
pub(super) struct SiteCompiler<'a> {
    pub(super) agents: Opened<'a>,
    pub(super) sites: BTreeMap<String, SiteFacts>,
    pub(super) boundary: Boundary,
    pub(super) engine: &'a Executable,
    pub(super) dialect: Option<&'a Dialect>,
    pub(super) charters: &'a Charters,
    pub(super) roots: &'a [PathBuf],
    pub(super) case_origin: &'a BTreeMap<String, usize>,
    /// The run's working directory, where a declared route overlay's
    /// `--patch` value is spelled relative to.
    pub(super) workdir: &'a Path,
}

/// What a body inherits from its seat: the layer that wrote it, and the
/// seat's results and bindings. A select case is written by its own
/// layer, and a panel step declares its own results. Every review-panel
/// member's word is bound to the seat's `residual` pair (decision 0109
/// ruling 2), whatever contains the member, and every sequence step knows
/// whether the seat declares `change`, a select case's included (#1221 E).
#[derive(Clone, Copy)]
pub(super) struct SeatScope<'s> {
    pub(super) dir: &'s Path,
    pub(super) results: &'s [String],
    pub(super) secrets: &'s [String],
    pub(super) residual: ResidualPair,
    pub(super) declares_change: bool,
}

/// Whether a compiled input set declares `change`, the seat's own
/// identifier of the change a sequence carries on.
pub(super) fn declares_change(inputs: &[String]) -> bool {
    inputs.iter().any(|input| input == "change")
}

impl<'a> SiteCompiler<'a> {
    /// A compiler with no site written yet.
    pub(super) fn new(
        agents: Opened<'a>,
        (boundary, engine): (Boundary, &'a Executable),
        dialect: Option<&'a Dialect>,
        charters: &'a Charters,
        roots: &'a [PathBuf],
        case_origin: &'a BTreeMap<String, usize>,
        workdir: &'a Path,
    ) -> SiteCompiler<'a> {
        SiteCompiler {
            agents,
            sites: BTreeMap::new(),
            boundary,
            engine,
            dialect,
            charters,
            roots,
            case_origin,
            workdir,
        }
    }

    /// A compiler under the same law with nothing opened and no site
    /// written, whose facts the compile drops.
    pub(super) fn scratch(&self) -> SiteCompiler<'a> {
        let law = (self.boundary, self.engine);
        let agents = self.agents.fresh();
        SiteCompiler::new(
            agents,
            law,
            self.dialect,
            self.charters,
            self.roots,
            self.case_origin,
            self.workdir,
        )
    }

    /// A seat's body, in the form `classify` read: the agent it resolved,
    /// if it names one, and a sequence's whole-gate fact, if it is one. A
    /// seat that declares secret bindings opens the adapters first, which
    /// say what route may carry them (decision 0021).
    pub(super) fn parse_seat_body(
        &mut self,
        at: SeatScope<'_>,
        phase: &str,
        raw: &Value,
        shape: SeatShape,
    ) -> Result<
        (
            Option<ResolvedSeat>,
            SeatBody,
            Option<super::GateAssessment>,
        ),
        CompileError,
    > {
        if raw.get("secrets").is_some() {
            self.agents.adapters()?;
        }
        match shape {
            SeatShape::Agent => {
                let agent_seat =
                    resolve_reference(self, at.dir, phase, raw, at.secrets, SEAT.role)?;
                refuse_results_beyond_agent(phase, at.results, &agent_seat)?;
                let body = SeatBody::Invocation(Invocation::Single {
                    role_path: agent_seat.role_path.clone(),
                    command: agent_seat.command.clone(),
                    candidates: agent_seat.candidates.clone(),
                    written: Vec::new(),
                    class: parse_class(phase, raw)?,
                });
                Ok((Some(agent_seat), body, None))
            }
            SeatShape::Panel => {
                let (members, aggregate) = self.parse_panel(at, phase, raw)?;
                let body = SeatBody::Invocation(Invocation::Panel { members, aggregate });
                Ok((None, body, None))
            }
            SeatShape::Sequence => {
                let (steps, whole_gate) = parse_sequence(self, at, phase, raw)?;
                Ok((None, SeatBody::Sequence { steps }, Some(whole_gate)))
            }
            SeatShape::Select => Ok((None, self.parse_select(at, phase, raw)?, None)),
            SeatShape::Inline => Ok((None, self.parse_inline_seat(at, phase, raw)?, None)),
        }
    }

    /// A seat written inline: its command as written, then its role, its
    /// command and its class, in that order.
    fn parse_inline_seat(
        &mut self,
        at: SeatScope<'_>,
        phase: &str,
        raw: &Value,
    ) -> Result<SeatBody, CompileError> {
        let written = self.inline_written(at, phase, raw, true)?;
        Ok(SeatBody::Invocation(Invocation::Single {
            role_path: parse_role(at.dir, phase, raw, self.charters, &mut self.sites)?,
            command: parse_command((self.engine, at.dir), phase, raw, at.secrets)?,
            candidates: Vec::new(),
            written,
            class: parse_class(phase, raw)?,
        }))
    }

    /// A select seat: every case under the layer that wrote it, and the
    /// default under the seat's own.
    pub(super) fn parse_select(
        &mut self,
        at: SeatScope<'_>,
        phase: &str,
        raw: &Value,
    ) -> Result<SeatBody, CompileError> {
        let (cases_raw, default_raw) = select_cases(phase, raw).map_err(select_refused)?;
        let roots = self.roots;
        let mut cases = BTreeMap::new();
        for (case, body) in cases_raw {
            let label = format!("{phase}:{case}");
            let dir = &roots[self.case_origin.get(&label).copied().unwrap_or(0)];
            let case_at = SeatScope { dir, ..at };
            cases.insert(case.clone(), self.parse_case(case_at, &label, body)?);
        }
        let default = default_raw
            .map(|body| {
                self.parse_case(at, &format!("{phase}:default"), body)
                    .map(Box::new)
            })
            .transpose()?;
        Ok(SeatBody::Select { cases, default })
    }

    /// Parse one body inside a selector. The label includes the case name,
    /// so every existing refusal identifies both the seat and the bad case.
    fn parse_case(
        &mut self,
        at: SeatScope<'_>,
        what: &str,
        raw: &Value,
    ) -> Result<SeatBody, CompileError> {
        match classify(what, raw, &CASE)? {
            CaseShape::Agent => self.parse_agent_case(at, what, raw),
            CaseShape::Panel => {
                let (members, aggregate) = self.parse_panel(at, what, raw)?;
                refuse_class_without_a_driver(what, raw)?;
                Ok(SeatBody::Invocation(Invocation::Panel {
                    members,
                    aggregate,
                }))
            }
            CaseShape::Sequence => {
                // Its steps say its fact: a_selected_sequence_holds_no_dialect_step.
                let (steps, _) = parse_sequence(self, at, what, raw)?;
                refuse_class_without_a_driver(what, raw)?;
                Ok(SeatBody::Sequence { steps })
            }
            CaseShape::Inline => self.parse_inline_case(at, what, raw),
        }
    }

    /// A case that names an agent, judged at its own site.
    fn parse_agent_case(
        &mut self,
        at: SeatScope<'_>,
        what: &str,
        raw: &Value,
    ) -> Result<SeatBody, CompileError> {
        let resolved = resolve_reference(self, at.dir, what, raw, at.secrets, CASE.role)?;
        refuse_results_beyond_agent(what, at.results, &resolved)?;
        let class = self.judge(at, what, raw, &resolved.candidates, resolved.hands)?;
        Ok(SeatBody::Invocation(Invocation::Single {
            role_path: resolved.role_path,
            command: resolved.command,
            candidates: resolved.candidates,
            written: Vec::new(),
            class,
        }))
    }

    /// A case written inline, judged before its role and command are read.
    fn parse_inline_case(
        &mut self,
        at: SeatScope<'_>,
        what: &str,
        raw: &Value,
    ) -> Result<SeatBody, CompileError> {
        let written = self.inline_written(at, what, raw, false)?;
        let class = self.judge(at, what, raw, &[], None)?;
        Ok(SeatBody::Invocation(Invocation::Single {
            role_path: parse_role(at.dir, what, raw, self.charters, &mut self.sites)?,
            command: parse_command((self.engine, at.dir), what, raw, at.secrets)?,
            candidates: Vec::new(),
            written,
            class,
        }))
    }

    /// Parse a panel body (`"panel": {…members…}, "aggregate": "…"`) at
    /// `what` — a seat's phase or a sequence step's `<phase>:<step>` label.
    /// `at.results` is checked to cover the aggregate's vocabulary: the
    /// seat's own for a seat-level panel (its aggregate reaches decide()),
    /// and a panel STEP's own — the seat's only when it is the FINAL step,
    /// because a non-final step's aggregate output never reaches decide(),
    /// it only feeds later steps as context.
    pub(super) fn parse_panel(
        &mut self,
        at: SeatScope<'_>,
        what: &str,
        raw: &Value,
    ) -> Result<(Vec<PanelMember>, Aggregate), CompileError> {
        let members_raw = raw.get("panel").and_then(Value::as_object).ok_or_else(|| {
            CompileError::Invalid(format!("seat '{what}' panel must be an object"))
        })?;
        if members_raw.len() < 2 {
            return Err(CompileError::Invalid(format!(
                "seat '{what}' panel needs at least two members; \
                 a one-member panel is a single seat"
            )));
        }
        refuse_mixed_panel(what, members_raw)?;
        let aggregate_name = raw
            .get("aggregate")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                CompileError::Invalid(format!("seat '{what}' panel needs 'aggregate'"))
            })?;
        let aggregate = Aggregate::parse(aggregate_name).ok_or_else(|| {
            CompileError::Invalid(format!(
                "seat '{what}' unknown aggregate '{aggregate_name}'; known: {}",
                Aggregate::known()
            ))
        })?;
        (aggregate.refuse_unbindable(what, at.residual)).map_err(CompileError::Body)?;
        for required in aggregate.required_results() {
            if !at.results.iter().any(|r| r == required) {
                return Err(CompileError::Invalid(format!(
                    "seat '{what}' aggregate '{aggregate_name}' can emit \
                     '{required}' but the seat does not declare it"
                )));
            }
        }
        let mut members = Vec::with_capacity(members_raw.len());
        for (name, member_raw) in members_raw {
            members.push(self.parse_member(at, (what, aggregate), name, member_raw)?);
        }
        Ok((members, aggregate))
    }

    /// One panel member, `<what>:<name>`, judged at its own site, with the
    /// result set its hired agent declares under the panel's `aggregate`
    /// (decision 0109 ruling 11): read from the agent, never from a key
    /// of the member's own, which [`MEMBER`] refuses.
    fn parse_member(
        &mut self,
        at: SeatScope<'_>,
        (what, aggregate): (&str, Aggregate),
        name: &str,
        raw: &Value,
    ) -> Result<PanelMember, CompileError> {
        let site = format!("{what}:{name}");
        let shape = classify(&site, raw, &MEMBER)?;
        let (role_path, command, candidates, agent_hands, written, declared) = match shape {
            MemberShape::Inline => {
                let written = self.inline_written(at, &site, raw, false)?;
                (
                    parse_role(at.dir, &site, raw, self.charters, &mut self.sites)?,
                    parse_command((self.engine, at.dir), &site, raw, at.secrets)?,
                    Vec::new(),
                    None,
                    written,
                    None,
                )
            }
            MemberShape::Agent => {
                let resolved =
                    resolve_reference(self, at.dir, &site, raw, at.secrets, MEMBER.role)?;
                (
                    resolved.role_path,
                    resolved.command,
                    resolved.candidates,
                    resolved.hands,
                    command_parts(raw),
                    resolved.results,
                )
            }
        };
        let class = self.judge(at, &site, raw, &candidates, agent_hands)?;
        Ok(PanelMember {
            name: name.to_string(),
            role_path,
            command,
            candidates,
            written,
            class,
            law: MemberLaw {
                results: aggregate.member_results(declared),
                residual: at.residual,
            },
        })
    }

    /// An inline site's command as written, once its typed `tools` are
    /// judged against it and recorded. Typed `tools` open the adapters
    /// strictly; a built-in model driver opens them by the optional read.
    pub(super) fn inline_written(
        &mut self,
        at: SeatScope<'_>,
        what: &str,
        raw: &Value,
        seat: bool,
    ) -> Result<Vec<String>, CompileError> {
        let written = command_parts(raw);
        if raw.get("tools").is_some() {
            self.agents.adapters()?;
        }
        if built_in_model_driver(&written).is_some() {
            self.agents.read_for_inline_driver();
        }
        let adapters = self.agents.loaded();
        let expand = (self.engine, at.dir);
        record_inline_tools(expand, what, raw, seat, &written, adapters, &mut self.sites)?;
        Ok(written)
    }

    /// Decision 0021 and the hands record at one driver-bearing site, under
    /// the agent's hands when it names one: the site's class.
    pub(super) fn judge(
        &mut self,
        at: SeatScope<'_>,
        what: &str,
        raw: &Value,
        candidates: &[Candidate],
        agent_hands: Option<HandsSpec>,
    ) -> Result<SeatClass, CompileError> {
        let law = SiteLaw {
            boundary: self.boundary,
            dir: at.dir,
            agent_hands: agent_hands.as_ref(),
        };
        let class = enforce_model_policy(
            what,
            raw,
            candidates,
            at.secrets,
            &mut self.agents,
            law,
            &mut self.sites,
        )?;
        record_hands(what, raw, agent_hands, at.secrets, &mut self.sites)?;
        Ok(class)
    }
}

/// A select's closed vocabulary, judged before any case is parsed: its
/// keys, its `on`, its strategy cases, and a body for every strategy.
/// Returns the cases and the default.
fn select_cases<'v, 'a>(
    phase: &'a str,
    raw: &'v Value,
) -> Result<(&'v Map<String, Value>, Option<&'v Value>), SelectRefusal<'a, 'v>> {
    let select = raw
        .get("select")
        .and_then(Value::as_object)
        .ok_or(SelectRefusal::NotObject { phase })?;
    for key in select.keys() {
        if !["on", "cases", "default"].contains(&key.as_str()) {
            return Err(SelectRefusal::UnknownKey { phase, key });
        }
    }
    match select.get("on").and_then(Value::as_str) {
        Some("strategy") => {}
        other => {
            let on = other.unwrap_or("<missing>");
            return Err(SelectRefusal::UnknownOn { phase, on });
        }
    }
    let cases_raw = select
        .get("cases")
        .and_then(Value::as_object)
        .ok_or(SelectRefusal::CasesNotObject { phase })?;
    for case in cases_raw.keys() {
        if !SELECT_STRATEGIES.contains(&case.as_str()) {
            return Err(SelectRefusal::UnknownCase { phase, case });
        }
    }
    let has_default = select.get("default").is_some();
    for strategy in SELECT_STRATEGIES {
        if !cases_raw.contains_key(strategy) && !has_default {
            return Err(SelectRefusal::Unresolved { phase, strategy });
        }
    }
    Ok((cases_raw, select.get("default")))
}

/// Why a select seat's closed vocabulary was refused, one variant per
/// check of [`select_cases`], in its order. `phase` borrows the seat's
/// name and every other field the select itself; the `Display` is the
/// operator's text byte for byte, and [`select_refused`] is the one place
/// it becomes a [`CompileError`] (#353).
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
enum SelectRefusal<'a, 'v> {
    /// `select` is absent or not an object.
    #[error("seat '{phase}' select must be an object")]
    NotObject { phase: &'a str },
    /// The first key outside `on`, `cases` and `default`, in map order.
    #[error("seat '{phase}' select has unknown key '{key}'; known: on, cases, default")]
    UnknownKey { phase: &'a str, key: &'v str },
    /// `on` is not `strategy`; `<missing>` when absent or not a string.
    #[error(
        "seat '{phase}' select has unknown 'on' {on}; known: strategy — the selection vocabulary is closed"
    )]
    UnknownOn { phase: &'a str, on: &'v str },
    /// `cases` is absent or not an object.
    #[error("seat '{phase}' select.cases must be an object")]
    CasesNotObject { phase: &'a str },
    /// The first case outside [`SELECT_STRATEGIES`], in map order.
    #[error(
        "seat '{phase}' select has unknown strategy case '{case}'; known: {}",
        SELECT_STRATEGIES.join(", ")
    )]
    UnknownCase { phase: &'a str, case: &'v str },
    /// The first strategy, in [`SELECT_STRATEGIES`]'s order, with no case
    /// and no default.
    #[error(
        "strategy class '{strategy}' resolves to nothing in seat '{phase}'; add that case or a default"
    )]
    Unresolved {
        phase: &'a str,
        strategy: &'static str,
    },
}

/// A select's refusal as the compile reports it.
fn select_refused(refusal: SelectRefusal<'_, '_>) -> CompileError {
    CompileError::Invalid(refusal.to_string())
}

/// A panel may judge or work, never do both: its members' written classes
/// must agree.
fn refuse_mixed_panel(what: &str, members_raw: &Map<String, Value>) -> Result<(), CompileError> {
    let gate_members: Vec<&str> = members_raw
        .iter()
        .filter(|(_, member)| member.get("class").and_then(Value::as_str) == Some("gate"))
        .map(|(name, _)| name.as_str())
        .collect();
    let work_members: Vec<&str> = members_raw
        .iter()
        .filter(|(_, member)| member.get("class").and_then(Value::as_str) != Some("gate"))
        .map(|(name, _)| name.as_str())
        .collect();
    if !gate_members.is_empty() && !work_members.is_empty() {
        return Err(CompileError::Invalid(format!(
            "seat '{what}' is a mixed panel: gate members [{}], work members [{}]; \
             a panel may judge or work, never do both",
            gate_members.join(", "),
            work_members.join(", ")
        )));
    }
    Ok(())
}

/// A single seat's declared `results` lie within its hired agent's list
/// where the agent declares one (decision 0109 ruling 11): the list is a
/// ceiling, never a floor, and an agent that declares none leaves the
/// seat as it is.
fn refuse_results_beyond_agent(
    what: &str,
    results: &[String],
    hired: &ResolvedSeat,
) -> Result<(), CompileError> {
    let Some(ceiling) = &hired.results else {
        return Ok(());
    };
    match results.iter().find(|word| !ceiling.contains(word)) {
        None => Ok(()),
        Some(word) => Err(CompileError::Invalid(format!(
            "seat '{what}' declares result '{word}', which its hired agent's results [{}] \
             do not admit; an agent's results are the ceiling of every seat that hires it \
             (decision 0109 ruling 11)",
            ceiling.join(", ")
        ))),
    }
}

#[cfg(test)]
mod tests;
