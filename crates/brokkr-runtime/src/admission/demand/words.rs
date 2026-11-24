//! A demand reason's kind, the word scripts read, and its text, which
//! names the resource and its numbers (decision 0125 ruling 6): the
//! demand as its average and margin, what is available, and what is
//! reserved and by how many runs.

use std::fmt;

use super::{Holder, Reserved, Short, Sized, Source};

impl Short {
    /// The reason's kind, as scripts read it.
    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Short::Memory { .. } => "memory_short",
            Short::Cpu { .. } => "cpu_short",
            Short::Unknown { .. } | Short::Held(_) => "demand_unknown",
            Short::Unreadable { .. } => "journal_unreadable",
        }
    }
}

/// Where a demand's average came from, as the operator reads it.
fn source(source: Source) -> String {
    match source {
        Source::Measured { runs } => format!("the average of its last {runs} measured runs"),
        Source::Declared => "its declared demand".to_string(),
        Source::Unknown => "no measured run and no declaration".to_string(),
    }
}

/// What is reserved, and by what.
fn reserved(reserved: &Reserved, unit: &str, amount: u64) -> String {
    let whole = match reserved.whole {
        0 => String::new(),
        whole => format!(", {whole} of them unmeasured and reserved whole"),
    };
    let admitted = match reserved.admitted {
        0 => String::new(),
        admitted => format!(", and by {admitted} entries admitted before it in this pass"),
    };
    format!(
        "{amount} {unit} reserved by {} running runs{whole}{admitted}",
        reserved.runs
    )
}

/// One resource's shortfall: `name`, its unit, the entry's demand, what
/// is available and reserved, and the whole machine where the demand is
/// more than it.
struct Shortfall<'a> {
    name: &'static str,
    unit: &'static str,
    sized: &'a Sized,
    need: super::Need,
    available: u64,
    total: u64,
    reserved: u64,
}

impl fmt::Display for Shortfall<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Shortfall {
            name,
            unit,
            sized,
            need,
            ..
        } = self;
        let demand = format!(
            "{} {unit} ({} average over {} + {} margin)",
            need.demand,
            need.average,
            source(sized.source),
            need.margin
        );
        let reserved = reserved(&sized.reserved, unit, self.reserved);
        write!(
            f,
            "{name} is short: recipe {} needs {demand}, {} {unit} available, {reserved}",
            sized.recipe, self.available
        )
    }
}

/// The text of a demand larger than the whole machine.
fn whole(f: &mut fmt::Formatter<'_>, shortfall: &Shortfall<'_>) -> fmt::Result {
    write!(
        f,
        "{shortfall}; it needs more than the machine's {} {}, so it starts only when nothing \
         else runs",
        shortfall.total, shortfall.unit
    )
}

impl fmt::Display for Short {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Short::Memory { sized, whole: big } => {
                let shortfall = Shortfall {
                    name: "memory",
                    unit: "bytes",
                    sized,
                    need: sized.memory_bytes,
                    available: sized.available.memory_bytes,
                    total: sized.available.memory_total,
                    reserved: sized.reserved.memory_bytes,
                };
                match big {
                    true => whole(f, &shortfall),
                    false => write!(f, "{shortfall}"),
                }
            }
            Short::Cpu { sized, whole: big } => {
                let shortfall = Shortfall {
                    name: "cpu",
                    unit: "millicores",
                    sized,
                    need: sized.load_millicores,
                    available: sized.available.idle_millicores,
                    total: sized.available.cores_millicores,
                    reserved: sized.reserved.load_millicores,
                };
                match big {
                    true => whole(f, &shortfall),
                    false => write!(f, "{shortfall}"),
                }
            }
            Short::Unknown { recipe, beside } => write!(
                f,
                "recipe {recipe} has no measured or declared demand, so it starts only \
                 when nothing else runs, and {} runs are running, {} entries were admitted \
                 before it in this pass and {} journals of the realms map cannot be read",
                beside.running, beside.admitted, beside.unreadable
            ),
            Short::Held(holder) => write!(f, "{holder} holds the whole machine until it ends"),
            Short::Unreadable { journal, error } => write!(
                f,
                "the journal {} the realms map lists cannot be read, and sizes nothing: {error}",
                journal.display()
            ),
        }
    }
}

impl fmt::Display for Holder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Holder::Run { run, recipe } => write!(
                f,
                "running run '{run}' of recipe {recipe}, whose demand is unknown,"
            ),
            Holder::Entry { entry, recipe } => write!(
                f,
                "entry {entry} of recipe {recipe}, admitted before it in this pass with its \
                 demand unknown,"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{source, Source};

    /// Each source's words. An entry of unknown demand never reaches a
    /// shortfall's text (it waits as `demand_unknown`), but the match
    /// names every source.
    #[test]
    fn each_source_names_where_the_average_came_from() {
        let runs = Source::Measured { runs: 3 };
        assert_eq!(source(runs), "the average of its last 3 measured runs");
        assert_eq!(source(Source::Declared), "its declared demand");
        let unknown = source(Source::Unknown);
        assert_eq!(unknown, "no measured run and no declaration");
    }
}
