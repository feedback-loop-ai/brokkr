//! A line of text, or a string an event's reader consumes, read whole
//! against a harness's closed table of forms (#484). Its words are
//! normalised first: split on whitespace runs, each with its edge
//! punctuation trimmed and its case folded. A text is read only when its
//! words are one form's, one for one: anything else is unread, whatever
//! it names. Pure, like `measure`.

use super::read::{Class, Refused, Said};
use crate::probe::plan::REPLY;

/// One form a harness prints, its words written normalised, with what it
/// says. A word written `{...}` is a slot: any one word, and the object
/// of the refusal the form states. `{option}` holds a flag alone.
pub(super) struct Form {
    pub(super) words: &'static str,
    pub(super) says: Says,
}

/// What a form says.
#[derive(Clone, Copy)]
pub(super) enum Says {
    /// Nothing a fact rests on: a notice the harness prints on every
    /// launch, or a warning that refuses nothing.
    Nothing,
    /// A refusal of this class, whose object is the form's one slot.
    Refuses(Class),
    /// A refusal of this class, of this object, which the form names in
    /// its own words.
    RefusesThe(Class, &'static str),
}

/// commander's refusal of a flag it does not know, which claude and dsh
/// print; dsh's recorded 2026-10-03 as "error: unknown option '--model'".
pub(super) const UNKNOWN_OPTION: Form = Form {
    words: "error unknown option {option}",
    says: Says::Refuses(Class::Control),
};

/// One word of a text: folded for matching, and as spelled, its edge
/// punctuation trimmed, for the object a slot names.
struct Word {
    folded: String,
    spelled: String,
}

/// `text`'s words, normalised.
fn words(text: &str) -> Vec<Word> {
    let edge = |c: char| !(c.is_alphanumeric() || matches!(c, '-' | '_'));
    text.split_whitespace()
        .map(|word| word.trim_matches(edge))
        .filter(|word| !word.is_empty())
        .map(|word| Word {
            folded: word.to_lowercase(),
            spelled: word.to_string(),
        })
        .collect()
}

/// What `text` says, read whole: nothing when it is blank, the reply
/// when it is the reply the probe asks for, and what a form of `forms`
/// says when it is that form; `None` when it is none of them, a line of
/// punctuation alone among them.
pub(super) fn read(text: &str, forms: &[Form]) -> Option<Vec<Said>> {
    if text.trim().is_empty() {
        return Some(Vec::new());
    }
    let said = words(text);
    match said.as_slice() {
        [only] if only.folded == REPLY.to_lowercase() => Some(vec![Said::Reply]),
        _ => forms.iter().find_map(|form| matched(form, &said)),
    }
}

/// What `form` says of `said`, when the words are the form's one for one.
fn matched(form: &Form, said: &[Word]) -> Option<Vec<Said>> {
    let wanted: Vec<&str> = form.words.split_whitespace().collect();
    if wanted.len() != said.len() {
        return None;
    }
    let mut slots = Vec::new();
    for (want, word) in wanted.iter().zip(said) {
        let fits = match *want {
            "{option}" => word.spelled.starts_with('-'),
            slot if slot.starts_with('{') => true,
            literal => literal == word.folded,
        };
        if !fits {
            return None;
        }
        if want.starts_with('{') {
            slots.push(word.spelled.as_str());
        }
    }
    let refused = |class, object: String| vec![Said::Refusal(Refused { class, object })];
    Some(match form.says {
        Says::Nothing => Vec::new(),
        Says::Refuses(class) => refused(class, slots.concat()),
        Says::RefusesThe(class, object) => refused(class, object.to_string()),
    })
}
