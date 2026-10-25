//! brokkr-view's integration tests, linked as one binary (#423): each
//! file under tests/ is a module here, so its tests run as
//! `<file>::<name>`, and brokkr-cli's `test_targets` refuses a file that
//! is not.

#[path = "../../../tests/support/envelope.rs"]
mod envelope_builder;

// The journals several files hold the run view to, declared once.
#[path = "../../brokkr-core/tests/committed_journals.rs"]
mod committed_journals;
#[path = "../../../tests/support/seat_rows.rs"]
mod journals;

mod composed_view;
mod seat_items;
mod seat_rows;
