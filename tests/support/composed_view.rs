//! The run view composed from the trail, the seat rows and a journal page
//! (#753; decision 0089 slice 2b), held to the run view of the run read
//! whole: field for field and serialized byte for byte, each float by its
//! bits as its text spells them, all but each participant's checkpoints,
//! which the seat view pages, and the journal, whose pages, in order, are
//! today's rows. brokkr-view holds every committed and hand-built journal
//! to it, and brokkr-cli every fixture journal read from the store.

use brokkr_view::{JournalRow, RunView};
use serde_json::Value;

/// `value` as it is served, byte for byte: a float by its bits, as its
/// text spells them, where `Value`'s own `==` takes -0.0 for 0.0.
fn served(value: &Value) -> String {
    value.to_string()
}

/// `view` serialized, its participants' checkpoints and its journal taken
/// out.
fn composed_part(view: &RunView) -> Value {
    let mut value = serde_json::to_value(view).unwrap();
    value["journal"] = Value::Null;
    for participant in value["participants"].as_array_mut().unwrap() {
        participant["checkpoints"] = Value::Null;
    }
    value
}

/// The fields each element of two lists of objects of one length differs
/// in, as `run: field[at].key`.
fn elements(run: &str, field: &str, whole: &[Value], composed: &[Value]) -> Vec<String> {
    let mut found = Vec::new();
    for (at, (whole, composed)) in whole.iter().zip(composed).enumerate() {
        let keys = whole.as_object().into_iter().flat_map(|map| map.keys());
        let differ = keys.filter(|key| served(&whole[*key]) != served(&composed[*key]));
        found.extend(differ.map(|key| format!("{run}: {field}[{at}].{key}")));
    }
    found
}

/// Each field `composed` serializes otherwise than `whole`, a list's by its
/// element's place and field, as `run: field`.
fn fields(run: &str, whole: &RunView, composed: &RunView) -> Vec<String> {
    let (whole, composed) = (composed_part(whole), composed_part(composed));
    let mut found = Vec::new();
    for (field, value) in whole.as_object().unwrap() {
        let other = &composed[field];
        if served(value) == served(other) {
            continue;
        }
        let lists = value.as_array().zip(other.as_array());
        let lists = lists.filter(|(value, other)| value.len() == other.len());
        let differ = lists.map_or_else(Vec::new, |(value, other)| {
            elements(run, field, value, other)
        });
        if differ.is_empty() {
            found.push(format!("{run}: {field}"));
        }
        found.extend(differ);
    }
    found
}

/// Where `pages`, the run view composed with each of a run's journal pages
/// in order, is not `whole`, the run view of `run` read whole: each field
/// of a page's view that differs, once, then the journal where the pages'
/// rows, serialized and in order, are not its rows.
pub(crate) fn disagreements(run: &str, whole: &RunView, pages: &[RunView]) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for field in pages.iter().flat_map(|page| fields(run, whole, page)) {
        if !found.contains(&field) {
            found.push(field);
        }
    }
    let rows = |journal: &[JournalRow]| -> Vec<String> {
        let rows = journal
            .iter()
            .map(|row| serde_json::to_string(row).unwrap());
        rows.collect()
    };
    let paged: Vec<String> = pages.iter().flat_map(|page| rows(&page.journal)).collect();
    if paged != rows(&whole.journal) {
        found.push(format!("{run}: journal"));
    }
    found
}
