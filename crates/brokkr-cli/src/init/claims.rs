//! Everything a scaffold says of a box, a network or a tool bound, in one
//! place. The scaffold's README and the note verify journals are written
//! once, at `init`, so both follow the host `init` read: Brokkr's box,
//! and the network it denies, stand only under `namespace` (decision
//! 0046). Under every other word an exec script runs under no box of
//! Brokkr's, and nothing reports whether its network was narrowed, so
//! neither is claimed (#366). No other literal in `init` says either:
//! the scaffold's tests read every generated file on every host arm
//! against these words, and the living-docs guard reads their literals.

use super::{Boundary, Cli, Host};

/// The words that depend on the host arm `init` scaffolded for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Claims {
    /// What the README's `bundle.json` line says of the verify and ship
    /// gates.
    pub(super) gates: &'static str,
    /// How the README's `scripts/*.sh` line says verify runs.
    pub(super) runs: &'static str,
    /// The tool-grants paragraph's sentence on verify and ship.
    pub(super) scripts: &'static str,
    /// What follows "passed" in the note a passing verify journals. It
    /// stands inside a single-quoted `printf`, so it holds no `'`.
    pub(super) note: &'static str,
    /// The README's sentence on the `harness` boundary `realms.json`
    /// declares, and why; `None` where it declares none.
    pub(super) realm: Option<&'static str>,
}

/// Every claude seat the scaffold hires, on every host: `init` gives no
/// claude agent hands, so none is boxed, and a tool list pre-approves.
/// The scaffold's `realms.json` grants no capability, so each native
/// power the claude adapter declares is denied by name (decision 0065).
pub(super) const PRE_APPROVAL: &str = "Pre-approval removes no tool: an unboxed seat keeps the\n\
     harness's other defaults, your own permission settings and MCP\n\
     servers. Only `WebSearch` and `WebFetch` are denied, by name, because\n\
     `realms.json` grants neither.";

/// The claude-hired review gate, on every host: unboxed, under the
/// adapter's `acceptEdits`, and held to its charter by a HEAD check.
pub(super) const REVIEW_GATE: &str =
    "The review gate also runs unboxed under `acceptEdits`, as every\n\
     claude seat here does, so it can edit files: the engine parks a gate\n\
     that moves HEAD, and an edit it leaves uncommitted moves nothing.";

pub(super) const NAMESPACE: Claims = Claims {
    gates: "the verify and ship exec\n  gates, which `namespace` boxes",
    runs: "runs in Brokkr's box, without network",
    scripts: "Verify and ship are exec scripts with no model grant,\n\
              and `namespace` boxes them.",
    note: "boxed under namespace, with no network",
    realm: None,
};

const UNBOXED: Claims = Claims {
    gates: "the verify and ship exec\n  gates, which no box of Brokkr's holds",
    runs: "runs under no box of Brokkr's, with no network denial confirmed",
    scripts: "Verify and ship are exec scripts with no model grant,\n\
              and no box of Brokkr's holds them.",
    note: "unboxed, with no network denial confirmed",
    realm: None,
};

pub(super) const CODEX: Claims = Claims {
    realm: Some(
        "`realms.json` declares the `harness` boundary: codex holds each seat's \
         hands under its own sandbox — read-only for the review gate, \
         workspace-write for intake and implement — as `adapters/codex.json` \
         addresses it, and verify and ship run their pinned scripts under no box \
         of Brokkr's (decision 0046).",
    ),
    ..UNBOXED
};

pub(super) const MACOS: Claims = Claims {
    realm: Some(
        "`realms.json` declares the `harness` boundary: `namespace`, the default, \
         is built by bubblewrap 0.11 or newer, which is Linux-only, so on macOS \
         verify and ship run their pinned scripts under no box of Brokkr's \
         (decision 0046).",
    ),
    ..UNBOXED
};

/// The claims one host arm supports: `None` is the `namespace` default,
/// and `init` declares `harness` only for a codex scaffold or on macOS.
pub(super) fn claims(cli: Cli, boundary: Option<Boundary>) -> Claims {
    match boundary {
        None | Some(Boundary::Namespace) => NAMESPACE,
        Some(Boundary::Harness) => match cli {
            Cli::Codex => CODEX,
            Cli::Claude | Cli::Dsh => MACOS,
        },
        Some(Boundary::Seatbelt | Boundary::Container | Boundary::Open) => UNBOXED,
    }
}

impl Host {
    /// The claims the declared boundary and agent CLI support.
    pub(super) fn claims(&self) -> Claims {
        claims(self.cli, self.boundary)
    }
}
