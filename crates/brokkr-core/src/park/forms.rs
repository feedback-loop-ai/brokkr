//! The forms `contracts/park-cause.v1.schema.json` states, enforced by the
//! reader as the provider-wait reader enforces its own schema's: a cause
//! the schema refuses is a cause that does not read, refused by name
//! (decision 0106 ruling 1), never admitted because the type could hold it.
//!
//! The provider and route patterns are the provider-wait reader's own, so
//! the two contracts state one form once.

use super::error::FormError;
use crate::payload::{provider_form, route_form};

/// The longest `rule_id` the schema admits, in characters.
pub(crate) const RULE_ID_MAX: usize = 64;

/// A `detail` is a non-empty sentence.
pub(super) fn detail(value: &str) -> Result<(), FormError> {
    match value.is_empty() {
        true => Err(FormError::EmptyDetail),
        false => Ok(()),
    }
}

/// A `rule_id` is 1 to [`RULE_ID_MAX`] characters.
pub(super) fn rule_id(value: &str) -> Result<(), FormError> {
    let length = value.chars().count();
    match (1..=RULE_ID_MAX).contains(&length) {
        true => Ok(()),
        false => Err(FormError::RuleIdLength(length)),
    }
}

/// A `provider` is the provider-wait contract's provider form.
pub(super) fn provider(value: &str) -> Result<(), FormError> {
    match provider_form(value) {
        true => Ok(()),
        false => Err(FormError::Provider(value.to_string())),
    }
}

/// A `route` is the provider-wait contract's route form.
pub(super) fn route(value: &str) -> Result<(), FormError> {
    match route_form(value) {
        true => Ok(()),
        false => Err(FormError::Route(value.to_string())),
    }
}

/// A length in whole seconds is at least one.
pub(super) fn seconds(field: &'static str, value: u64) -> Result<(), FormError> {
    match value {
        0 => Err(FormError::Seconds(field)),
        _ => Ok(()),
    }
}
