//! `brokkr doctor`'s provisional section (proposed decision 0075 ruling
//! 5): each model an adapter declares `"tier": "provisional"`, and the
//! offices the world's `provisional_offices` lets it hold. Doctor
//! reports; the compile is what refuses.

use std::path::Path;

use brokkr_runtime::realms::World;
use brokkr_runtime::Adapters;

use super::Report;

/// One line per provisional model, or one line saying there is none. A
/// tree whose adapters will not load has no tier to read, and the agents
/// line already names why, so this section says nothing of its own.
pub(super) fn report_provisional(report: &mut Report, adapters_root: &Path, world: Option<&World>) {
    let Ok(adapters) = Adapters::load(adapters_root) else {
        return;
    };
    let offices = world.map_or(&[][..], |world| world.map.provisional_offices());
    let mut none = true;
    for adapter in adapters.providers() {
        for model in &adapter.provisional {
            none = false;
            let what = format!("provisional {model}");
            let on = format!("adapter '{}'", adapter.provider);
            match offices {
                [] => report.warn(
                    &what,
                    format!(
                        "{on} · may hold no office: realms.json lists no provisional_offices · \
                         never a gate"
                    ),
                ),
                listed => report.ok(
                    &what,
                    format!("{on} · may hold {} · never a gate", listed.join(", ")),
                ),
            }
        }
    }
    if none {
        report.ok(
            "provisional",
            "no adapter declares a provisional model".into(),
        );
    }
}

#[cfg(test)]
mod tests;
