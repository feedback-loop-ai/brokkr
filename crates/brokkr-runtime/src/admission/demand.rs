//! Decision 0125: an entry starts only when its recipe's measured demand,
//! plus what running runs still reserve, fits what the machine has
//! available now. Linux is measured ([`probe`]); a machine that is not,
//! or a pass given no [`Machine`], keeps decision 0068's count ceilings
//! exactly (ruling 2).
//!
//! A measured pass reads every journal of the realms map once
//! ([`journals`]), learns each recipe's expected peaks per phase from its
//! window ([`history`]), measures the machine and each running run's
//! driver tree once, and reserves for each running run what it may still
//! need (ruling 5). It then judges each entry the count checks admit, in
//! the queue's order, after every check of decision 0068 (ruling 6): an
//! entry it admits reserves its own demand before the next is judged, so
//! one headroom is never handed out twice in a pass. Nothing latches, and
//! nothing here asks a model or reads the network.

use std::collections::BTreeMap;
use std::path::PathBuf;

use brokkr_store::{EntryId, Store};
use serde::Serialize;

use super::capacity::{Host, Shared};
use super::host::demand::{Declared, Demand};
use super::host::Hosting;
use super::AdmissionError;
use history::{Peak, Profile};

mod history;
mod journals;
mod probe;
mod words;

pub use probe::{measurable, Available, Unmeasured};
pub(crate) use probe::{Driven, Measurement, Use};

/// The machine a measured pass sizes entries on, its effects injected:
/// every journal the realms map lists, and the measure of what the
/// machine has now and of each running run's driver tree.
pub struct Machine<'a> {
    journals: &'a [PathBuf],
    measure: &'a dyn Fn(&[Driven<'_>]) -> Measurement,
}

impl<'a> Machine<'a> {
    /// This machine, measured as [`probe`] measures it, beside `journals`.
    pub fn here(journals: &'a [PathBuf]) -> Machine<'a> {
        Machine::with(journals, &probe::measure_now)
    }

    /// A machine whose measures `measure` gives, as the tests inject them.
    pub(crate) fn with(
        journals: &'a [PathBuf],
        measure: &'a dyn Fn(&[Driven<'_>]) -> Measurement,
    ) -> Machine<'a> {
        Machine { journals, measure }
    }
}

/// Why decision 0125's rule holds an entry, or a journal it names beside
/// the queue, as its kind and its words say it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unfit(pub(crate) Short);

impl Unfit {
    pub(crate) fn kind(&self) -> &'static str {
        self.0.kind()
    }
}

impl std::fmt::Display for Unfit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// What an entry was sized by: its recipe, the demand judged and where it
/// came from, what the machine had, and what was reserved before it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Sized {
    pub recipe: String,
    pub source: Source,
    pub memory_bytes: Need,
    pub load_millicores: Need,
    pub available: Available,
    pub reserved: Reserved,
}

/// Where an entry's demand came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Source {
    /// The mean of the recipe's last `runs` measured runs.
    Measured { runs: usize },
    /// The host file's declaration, the recipe having no measured profile.
    Declared,
    /// Neither (ruling C).
    Unknown,
}

/// One resource's demand: the average, the margin over it, and their sum.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Need {
    pub average: u64,
    pub margin: u64,
    pub demand: u64,
}

impl Need {
    fn of(average: u64, margin_percent: u32) -> Need {
        let margin = u128::from(average) * u128::from(margin_percent);
        let margin = u64::try_from(margin.div_ceil(100)).unwrap_or(u64::MAX);
        Need {
            average,
            margin,
            demand: average.saturating_add(margin),
        }
    }
}

/// What is reserved before an entry: the memory and load, the running
/// runs that reserve them, how many of those were reserved whole because
/// their tree could not be read, and the entries admitted before it in
/// the pass.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Reserved {
    pub memory_bytes: u64,
    pub load_millicores: u64,
    pub runs: usize,
    pub whole: usize,
    pub admitted: usize,
}

/// A run, or an entry admitted earlier in the pass, whose demand is
/// unknown: it holds the whole machine until it ends (ruling 6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Holder {
    Run { run: String, recipe: String },
    Entry { entry: EntryId, recipe: String },
}

/// What else stands on the machine beside an entry of unknown demand.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Beside {
    pub(crate) running: usize,
    pub(crate) admitted: usize,
    pub(crate) unreadable: usize,
}

/// Why decision 0125's rule holds an entry, or, `Unreadable`, a journal
/// it names beside the queue. Each waits and clears by itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Short {
    /// `memory_short`; `whole` where the demand is more than the machine.
    Memory { sized: Box<Sized>, whole: bool },
    /// `cpu_short`; `whole` where the demand is more than every core.
    Cpu { sized: Box<Sized>, whole: bool },
    /// `demand_unknown`: no measured profile and no declaration, and something
    /// else stands on the machine.
    Unknown { recipe: String, beside: Beside },
    /// `demand_unknown`: a run or an admitted entry whose demand is
    /// unknown holds the machine.
    Held(Holder),
    /// `journal_unreadable`: a journal of the realms map, unread.
    Unreadable {
        journal: PathBuf,
        error: Shared<AdmissionError>,
    },
}

/// A pass's judge of demand: none where the pass is not measured.
pub(crate) enum Judge {
    Unmeasured,
    Measured(Box<Sizing>),
}

/// What a measured pass knows, and what it has reserved so far.
pub(crate) struct Sizing {
    machine: Available,
    profiles: BTreeMap<String, Profile>,
    settings: Demand,
    reserved: Reserved,
    holders: Vec<Holder>,
    running: usize,
    unreadable: Vec<Short>,
}

impl Judge {
    /// Read the journals and measure the machine, for the pass over
    /// `store` on `host`, whose host configuration reads as `hosting`.
    pub(crate) fn take(
        store: &Store,
        host: &Host<'_>,
        hosting: &Hosting,
    ) -> Result<Judge, AdmissionError> {
        let Some(machine) = host.machine else {
            return Ok(Judge::Unmeasured);
        };
        let settings = match hosting {
            Hosting::Declared(config) => config.demand.clone(),
            Hosting::Absent | Hosting::Unreadable(_) => return Ok(Judge::Unmeasured),
        };
        let candidates = settings.window_runs.get().saturating_mul(2);
        let read = journals::read(store, machine.journals, candidates)?;
        let driven: Vec<Driven<'_>> = (read.running.iter())
            .map(|running| Driven {
                journal: &running.journal,
                run: &running.run,
            })
            .collect();
        let Measurement::Measured { machine, runs } = (machine.measure)(&driven) else {
            return Ok(Judge::Unmeasured);
        };
        let mut sizing = Sizing {
            machine,
            profiles: history::learn(read.rows, settings.window_runs.get() as usize),
            settings,
            reserved: Reserved::default(),
            holders: Vec::new(),
            running: 0,
            unreadable: read.unreadable,
        };
        let uses = runs.into_iter().chain(std::iter::repeat(None));
        for (running, use_now) in read.running.into_iter().zip(uses) {
            sizing.reserve(running, use_now);
        }
        Ok(Judge::Measured(Box::new(sizing)))
    }

    /// Whether the pass is measured, so a host file need declare no run
    /// ceiling (ruling 7).
    pub(crate) fn measured(&self) -> bool {
        matches!(self, Judge::Measured(_))
    }

    /// Judge `entry`, of `recipe`, which every other check admits: what it
    /// was sized by, and whether it fits, reserving its demand when it
    /// does. An unmeasured pass sizes nothing and admits it.
    pub(crate) fn admit(
        &mut self,
        entry: EntryId,
        recipe: &str,
    ) -> (Option<Sized>, Result<(), Unfit>) {
        match self {
            Judge::Unmeasured => (None, Ok(())),
            Judge::Measured(sizing) => {
                let sized = sizing.sized(recipe);
                let fits = sizing.fits(entry, &sized).map_err(Unfit);
                (Some(sized), fits)
            }
        }
    }

    /// Each journal of the realms map the pass could not read, named once.
    pub(crate) fn unreadable(self) -> Vec<Unfit> {
        match self {
            Judge::Unmeasured => Vec::new(),
            Judge::Measured(sizing) => sizing.unreadable.into_iter().map(Unfit).collect(),
        }
    }
}

impl Sizing {
    /// A recipe's expected peaks over `phase` and the phases after it, and
    /// where they came from: its window, else its declaration.
    fn expected(&self, recipe: &str, phase: Option<&str>) -> Option<(Peak, Source)> {
        if let Some(profile) = self.profiles.get(recipe) {
            let source = Source::Measured { runs: profile.runs };
            return Some((profile.remaining(phase), source));
        }
        let declared: &Declared = self.settings.recipes.get(recipe)?;
        let peak = Peak {
            memory_bytes: declared.peak_memory_bytes.get(),
            load_millicores: declared.peak_load_millicores.get(),
        };
        Some((peak, Source::Declared))
    }

    /// Reserve for `running` what it may still need beyond `use_now`, or
    /// its whole expected peak where its tree was not read; a run whose
    /// demand is unknown holds the machine.
    fn reserve(&mut self, running: journals::Running, use_now: Option<Use>) {
        self.running += 1;
        let Some((peak, _)) = self.expected(&running.recipe, running.phase.as_deref()) else {
            self.holders.push(Holder::Run {
                run: running.run,
                recipe: running.recipe,
            });
            return;
        };
        let (memory, load) = match use_now {
            Some(used) => (
                peak.memory_bytes.saturating_sub(used.memory_bytes),
                peak.load_millicores.saturating_sub(used.load_millicores),
            ),
            None => {
                self.reserved.whole += 1;
                (peak.memory_bytes, peak.load_millicores)
            }
        };
        self.reserved.memory_bytes = self.reserved.memory_bytes.saturating_add(memory);
        self.reserved.load_millicores = self.reserved.load_millicores.saturating_add(load);
        self.reserved.runs += 1;
    }

    /// What an entry of `recipe` is sized by now: its whole expected peak
    /// and the margin.
    fn sized(&self, recipe: &str) -> Sized {
        let (peak, source) = self
            .expected(recipe, None)
            .unwrap_or((Peak::default(), Source::Unknown));
        let margin = self.settings.margin_percent;
        Sized {
            recipe: recipe.to_string(),
            source,
            memory_bytes: Need::of(peak.memory_bytes, margin),
            load_millicores: Need::of(peak.load_millicores, margin),
            available: self.machine,
            reserved: self.reserved,
        }
    }

    /// Nothing runs in any journal read, none is unreadable, and nothing
    /// was admitted before in the pass.
    fn idle(&self) -> bool {
        self.running == 0 && self.reserved.admitted == 0 && self.unreadable.is_empty()
    }

    /// Ruling 6, in its order, for `entry` as `sized`; a fit is reserved.
    fn fits(&mut self, entry: EntryId, sized: &Sized) -> Result<(), Short> {
        if sized.source == Source::Unknown {
            return self.alone(entry, sized);
        }
        if let Some(holder) = self.holders.first() {
            return Err(Short::Held(holder.clone()));
        }
        let memory = sized.memory_bytes.demand;
        let load = sized.load_millicores.demand;
        let whole_memory = memory > self.machine.memory_total;
        let whole_cpu = load > self.machine.cores_millicores;
        let short = |whole: bool, memory: bool| match memory {
            true => Short::Memory {
                sized: Box::new(sized.clone()),
                whole,
            },
            false => Short::Cpu {
                sized: Box::new(sized.clone()),
                whole,
            },
        };
        match (whole_memory || whole_cpu, self.idle()) {
            (true, true) => {}
            (true, false) => return Err(short(true, whole_memory)),
            (false, _) => {
                if memory.saturating_add(self.reserved.memory_bytes) > self.machine.memory_bytes {
                    return Err(short(false, true));
                }
                if load.saturating_add(self.reserved.load_millicores) > self.machine.idle_millicores
                {
                    return Err(short(false, false));
                }
            }
        }
        self.reserved.memory_bytes = self.reserved.memory_bytes.saturating_add(memory);
        self.reserved.load_millicores = self.reserved.load_millicores.saturating_add(load);
        self.reserved.admitted += 1;
        Ok(())
    }

    /// Ruling C: an entry of unknown demand starts only on an idle machine,
    /// and then holds it.
    fn alone(&mut self, entry: EntryId, sized: &Sized) -> Result<(), Short> {
        if !self.idle() {
            return Err(Short::Unknown {
                recipe: sized.recipe.clone(),
                beside: Beside {
                    running: self.running,
                    admitted: self.reserved.admitted,
                    unreadable: self.unreadable.len(),
                },
            });
        }
        self.holders.push(Holder::Entry {
            entry,
            recipe: sized.recipe.clone(),
        });
        self.reserved.admitted += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
