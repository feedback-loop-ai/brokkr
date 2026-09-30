//! What a scaffold may say stood around its verify and ship scripts. The
//! scaffold's README and the note verify journals are written once, at
//! `init`, so both follow the boundary the scaffold's realm declares:
//! Brokkr's box, and the network it denies, stand only under `namespace`
//! (decision 0046). Under every other word an exec script runs under no
//! box of Brokkr's, and nothing reports whether its network was narrowed,
//! so neither is claimed (#366).

use super::{Boundary, Host};

/// The three places the scaffold's words depend on the boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Claims {
    /// What the README calls the verify and ship gates.
    pub(super) gates: &'static str,
    /// How the README says verify runs.
    pub(super) runs: &'static str,
    /// What follows "passed" in the note a passing verify journals.
    pub(super) note: &'static str,
}

const BOXED: Claims = Claims {
    gates: "boxed exec",
    runs: "runs in Brokkr's box, without network",
    note: "with network denied",
};

const UNBOXED: Claims = Claims {
    gates: "unboxed exec",
    runs: "runs under no box of Brokkr's, with no network denial confirmed",
    note: "unboxed, with no network denial confirmed",
};

impl Host {
    /// The claims the declared boundary supports; `None` is the
    /// `namespace` default.
    pub(super) fn claims(&self) -> Claims {
        match self.boundary {
            None | Some(Boundary::Namespace) => BOXED,
            Some(Boundary::Seatbelt | Boundary::Container | Boundary::Harness | Boundary::Open) => {
                UNBOXED
            }
        }
    }
}
