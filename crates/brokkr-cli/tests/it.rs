//! brokkr-cli's integration tests, linked as one binary (#423): each file
//! under tests/ is a module here, so its tests run as `<file>::<name>`. A
//! file that needs a process of its own is its own `[[test]]` in
//! Cargo.toml instead, which names the reason, and `layering::test_targets`
//! refuses a file that is neither.

// Support shared by several files, declared once: a file reaches it as
// `crate::<name>`.
#[path = "support/numbered.rs"]
mod numbered;
#[path = "support/rust_source.rs"]
mod rust_source;
#[path = "support/test_paths.rs"]
mod test_paths;
#[path = "support/tracked.rs"]
mod tracked_files;
#[path = "support/tracked_text.rs"]
mod tracked_text;
#[path = "support/workflow.rs"]
mod workflow;
#[path = "support/workspace.rs"]
mod workspace_root;

mod agent_fallback;
mod agent_readouts;
mod binary_names;
mod bootstrap_bench;
mod boundary_readouts;
mod boundary_verbs;
mod budget_frame;
mod capability_broker;
mod capability_verbs;
mod contributing;
mod decisions_index;
mod delivered_by_brokkr;
mod diagrams;
mod doctor_dsh_selection;
mod dsh_sandbox_runner;
mod hands;
mod harness_probe;
mod hosts;
mod house_prose;
mod init_doctor;
mod init_stacks;
#[path = "layering/main.rs"]
mod layering;
mod machine_proof;
mod muninn;
mod mutants_gate;
mod packaging;
mod private_hosts;
mod provenance;
mod ratchets;
mod realms;
mod recipes;
mod rename_guard;
mod research_registry;
mod retired_overrides;
mod script_gates;
mod status_pages;
mod suppressions;
mod transcript_command;
mod transcript_privacy;
mod witness_journal;
mod workflow_pins;
