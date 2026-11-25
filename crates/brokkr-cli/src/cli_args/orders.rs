//! The work-order family's arguments (decision 0116 ruling 9).

use std::path::PathBuf;

use brokkr_core::work_order::WorkOrderId;

use super::JournalArgs;

/// Bare, the family takes the same arguments as its list verb.
#[derive(clap::Args)]
#[command(args_conflicts_with_subcommands = true)]
pub(crate) struct OrdersFamily {
    #[command(flatten)]
    pub(crate) list: OrderListArgs,
    #[command(subcommand)]
    pub(crate) command: Option<OrdersCmd>,
}

#[derive(clap::Subcommand)]
pub(crate) enum OrdersCmd {
    /// List work orders by number, with state, open ask count and title.
    /// Use `--json` for the view model.
    List(OrderListArgs),
    /// Print a work order's canonical Markdown, journaled state and text digest.
    Show(OrderShowArgs),
    /// Propose a work order from a Markdown file.
    Add(OrderAddArgs),
    /// Replace a work order's text and return it to proposed.
    Edit(OrderEditArgs),
    /// Rule the text identified by the digest from show, after every ask is closed.
    Rule(OrderRuleArgs),
}

#[derive(clap::Args)]
#[group(skip)]
pub(crate) struct OrderListArgs {
    #[command(flatten)]
    pub(crate) journal: JournalArgs,
    /// Emit the view model for scripts.
    #[arg(long)]
    pub(crate) json: bool,
}

#[derive(clap::Args)]
#[group(skip)]
pub(crate) struct OrderShowArgs {
    /// The work order's id.
    #[arg(value_name = "WO-<n>")]
    pub(crate) id: WorkOrderId,
    #[command(flatten)]
    pub(crate) journal: JournalArgs,
}

#[derive(clap::Args)]
#[group(skip)]
pub(crate) struct OrderAddArgs {
    /// The Markdown file holding the work order.
    pub(crate) file: PathBuf,
    #[command(flatten)]
    pub(crate) write: OrderWriteArgs,
}

#[derive(clap::Args)]
#[group(skip)]
pub(crate) struct OrderEditArgs {
    /// The work order's id (WO-<n>).
    pub(crate) id: WorkOrderId,
    /// The Markdown file holding the replacement text.
    pub(crate) file: PathBuf,
    #[command(flatten)]
    pub(crate) write: OrderWriteArgs,
}

#[derive(clap::Args)]
#[group(skip)]
pub(crate) struct OrderRuleArgs {
    /// The work order's id.
    #[arg(value_name = "WO-<n>")]
    pub(crate) id: WorkOrderId,
    /// The 64 lowercase hexadecimal characters printed by orders show.
    #[arg(long, value_name = "HEX", value_parser = read_digest)]
    pub(crate) digest: String,
    #[command(flatten)]
    pub(crate) write: OrderWriteArgs,
}

#[derive(Debug)]
struct InvalidDigest;

impl std::fmt::Display for InvalidDigest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("expected a digest of 64 lowercase hexadecimal characters")
    }
}

impl std::error::Error for InvalidDigest {}

fn read_digest(value: &str) -> Result<String, InvalidDigest> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    {
        Ok(value.to_owned())
    } else {
        Err(InvalidDigest)
    }
}

#[derive(clap::Args)]
#[group(skip)]
pub(crate) struct OrderWriteArgs {
    /// Why, journaled with the command.
    #[arg(long, value_name = "WHY")]
    pub(crate) reason: String,
    #[command(flatten)]
    pub(crate) journal: JournalArgs,
}
