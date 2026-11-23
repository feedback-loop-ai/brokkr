//! Decision 0125: an entry starts only when its recipe's measured demand,
//! plus what running runs still reserve, fits what the machine has
//! available now. Linux is measured ([`probe`]); a machine that is not
//! keeps decision 0068's count ceilings exactly (ruling 2).

mod probe;

pub use probe::{measurable, measure_now, Available, Driven, Measurement, Unmeasured, Use, WINDOW};
