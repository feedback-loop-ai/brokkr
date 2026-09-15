//! The bounded R3 lifetime feasibility probe for decision 0046 slice II.
//!
//! The shared model and its host-independent tests live in
//! [`seatbelt_probe`]. On macOS the same target additionally runs the real
//! per-invocation transient launchd lease-pair experiment against
//! `/usr/bin/sandbox-exec`; on every other host that experiment is not
//! compiled, and the model tests are not native evidence. A pass here does
//! not close SEATBELT-R3.

mod seatbelt_probe;
