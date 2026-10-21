//! The whole-gate fact and its first written work site (decision 0094).
use super::{Invocation, SeatBody, SeatClass, StepBody};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateAssessment {
    AllGate,
    ContainsWork(WorkSite),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkSite {
    pub path: Vec<String>,
    pub compiled: bool,
}

impl WorkSite {
    pub fn seat() -> Self {
        Self {
            path: vec![],
            compiled: true,
        }
    }

    fn under(mut self, name: &str) -> Self {
        self.path.insert(0, name.into());
        self
    }
}

impl GateAssessment {
    pub(super) fn first_work(&mut self, site: WorkSite) {
        if matches!(self, Self::AllGate) {
            *self = Self::ContainsWork(site);
        }
    }

    fn under(self, name: &str) -> Self {
        match self {
            Self::AllGate => Self::AllGate,
            Self::ContainsWork(site) => Self::ContainsWork(site.under(name)),
        }
    }
}

fn invocation_assessment<C>(invocation: &Invocation<C>, class: SeatClass) -> GateAssessment {
    match invocation {
        Invocation::Single { .. } => match class {
            SeatClass::Gate => GateAssessment::AllGate,
            SeatClass::Work => GateAssessment::ContainsWork(WorkSite::seat()),
        },
        Invocation::Panel { members, .. } => panel_assessment(members),
    }
}

pub(super) fn panel_assessment(members: &[super::PanelMember]) -> GateAssessment {
    members
        .iter()
        .find(|member| member.class == SeatClass::Work)
        .map_or(GateAssessment::AllGate, |member| {
            GateAssessment::ContainsWork(WorkSite::seat().under(&member.name))
        })
}

pub(super) fn step_assessment(name: &str, class: SeatClass, body: &StepBody) -> GateAssessment {
    match body {
        StepBody::Invocation(invocation) => invocation_assessment(invocation, class),
        StepBody::Dialect { .. } => match class {
            SeatClass::Gate => GateAssessment::AllGate,
            SeatClass::Work => GateAssessment::ContainsWork(WorkSite::seat()),
        },
    }
    .under(name)
}

impl SeatBody {
    /// An empty body cannot reach this walk: removal controls are
    /// panel_and_sequence_parsers_refuse_every_ambiguous_shape and
    /// select_parses_every_case_and_refuses_closed_vocabulary_defects_by_case
    /// in bundle::tests (decision 0094; decision 0071 ruling 9).
    pub(super) fn assess(&self) -> GateAssessment {
        match self {
            SeatBody::Invocation(invocation) => {
                let class = match invocation {
                    Invocation::Single { class, .. } => *class,
                    Invocation::Panel { .. } => SeatClass::Gate,
                };
                invocation_assessment(invocation, class)
            }
            SeatBody::Sequence { steps } => steps
                .iter()
                .map(|step| step_assessment(&step.name, step.class, &step.body))
                .find(|assessment| matches!(assessment, GateAssessment::ContainsWork(_)))
                .unwrap_or(GateAssessment::AllGate),
            SeatBody::Select { cases, default } => cases
                .iter()
                .map(|(name, body)| body.assess().under(name))
                .chain(default.iter().map(|body| body.assess().under("default")))
                .find(|assessment| matches!(assessment, GateAssessment::ContainsWork(_)))
                .unwrap_or(GateAssessment::AllGate),
        }
    }
}
