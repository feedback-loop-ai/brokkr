//! Entry is derived once, then painted by every surface (decision 0094).
use brokkr_core::{entry::RunEntry, EventEnvelope};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EntryView {
    Entered {
        phase: String,
        continues: Vec<ContinuedView>,
        text: String,
    },
    Unreadable {
        error: String,
        text: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ContinuedView {
    pub title: String,
    pub feature: String,
    pub run_id: String,
}

pub fn entry_sentence(entry: &RunEntry) -> String {
    let chain = if entry.continues.is_empty() {
        "no earlier run".into()
    } else {
        entry
            .continues
            .iter()
            .map(|run| format!("{} ({})", crate::title(&run.feature), run.run_id))
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!("entered at {}, continuing {chain}", entry.phase)
}

pub(super) fn of_events(events: &[EventEnvelope]) -> Option<EntryView> {
    match RunEntry::of_started(&events.first()?.payload) {
        Ok(None) => None,
        Ok(Some(entry)) => Some(EntryView::Entered {
            phase: entry.phase.clone(),
            text: entry_sentence(&entry),
            continues: entry
                .continues
                .into_iter()
                .map(|run| ContinuedView {
                    title: crate::title(&run.feature),
                    feature: run.feature,
                    run_id: run.run_id,
                })
                .collect(),
        }),
        Err(error) => Some(EntryView::Unreadable {
            text: format!("entry unreadable: {error}"),
            error: error.to_string(),
        }),
    }
}
