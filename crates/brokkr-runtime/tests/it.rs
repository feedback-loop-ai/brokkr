//! brokkr-runtime's integration tests, linked as one binary (#423): each
//! file under tests/ is a module here, so its tests run as
//! `<file>::<name>`. A file that needs a process of its own is its own
//! `[[test]]` in Cargo.toml instead, which names the reason, and
//! brokkr-cli's `test_targets` refuses a file that is neither.

// Support shared by several files, declared once: a file reaches it as
// `crate::witnesses`.
#[path = "support/witnesses.rs"]
mod witnesses;

mod adoption;
mod anchor_test;
mod budgets;
mod crucible_review_sequence;
mod dispatch_seam;
mod fenced_commands;
mod frozen_contracts;
mod gpt_flash_shape;
mod inline_resume;
mod landing_shape;
mod library_data;
mod node_recipe_gates;
mod preflight_shape;
mod recipe_library;
mod recovery;
mod release_shape;
mod roster;
mod sdd_progress;
mod sdd_shape;
mod table_lints;
mod triage_shape;
mod two_engines_one_journal;
mod wager_parity;
mod witness_digests;
