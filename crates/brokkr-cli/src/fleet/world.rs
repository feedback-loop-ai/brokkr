//! `brokkr runs` over a many-hearth world: each hearth's runs under its
//! realm (decision 0026 ruling 3), read a hearth at a time, in the
//! hearths' order, and each hearth a page at a time as its one-hearth
//! listing reads it (decision 0089 ruling 6). Its memory is one hearth's
//! listing however many hearths the world holds, and the shape it prints
//! is the [`brokkr_view::FleetView`] it always printed.

use std::io::Write;

use anyhow::Result;
use brokkr_runtime::realms::Hearth;
use brokkr_view::VIEW_VERSION;

use super::listing::{close_array, item_break, list_journal, Filter, Reach};
use super::page::TAIL_KEYS;
use crate::render::ending::Tally;
use crate::render::{self, Style};

/// Every run of every hearth of `hearths` at `now` that `filter` keeps,
/// written to `out`: the fleet view as JSON when `json` asks, else the
/// table under each realm.
pub(crate) fn list_world(
    hearths: &[Hearth],
    (json, filter): (bool, Filter),
    clock: (&str, &Style),
    out: &mut impl Write,
) -> Result<()> {
    list_world_in(hearths, (json, TAIL_KEYS, filter), clock, out)
}

/// [`list_world`], `keys` of each hearth's runs behind past a refresh
/// keyed at a time.
pub(crate) fn list_world_in(
    hearths: &[Hearth],
    (json, keys, filter): (bool, usize, Filter),
    (now, style): (&str, &Style),
    out: &mut impl Write,
) -> Result<()> {
    let (labels, journals) = super::names(hearths);
    let named = hearths.iter().zip(labels.iter().zip(&journals));
    let named: Vec<Named<'_>> = named
        .map(|(hearth, (realm, journal))| Named {
            hearth,
            realm,
            journal,
        })
        .collect();
    if json {
        world_json(&named, ((now, keys), filter), out)
    } else {
        world_text(&named, ((now, keys), filter), style, out)
    }
}

/// How a world reads each hearth: at `now`, `keys` of its runs behind
/// keyed at a time, listing the runs the filter keeps.
type Reading<'a> = ((&'a str, usize), Filter);

/// One hearth, and the names the listing gives it.
struct Named<'a> {
    hearth: &'a Hearth,
    realm: &'a str,
    journal: &'a str,
}

/// The fleet view, pretty, written a hearth and a page of rows at a time.
fn world_json(named: &[Named<'_>], read: Reading<'_>, out: &mut impl Write) -> Result<()> {
    let open = format!("{{\n  \"view_version\": {VIEW_VERSION},\n  \"realms\": [");
    out.write_all(open.as_bytes())?;
    let mut count = 0;
    for (index, hearth) in named.iter().enumerate() {
        out.write_all(item_break(index))?;
        count += realm_json(hearth, read, out)?;
    }
    close_array(out, named.len(), "  ")?;
    writeln!(out, ",\n  \"count\": {count}\n}}")?;
    Ok(())
}

/// One hearth's realm in the fleet view, pretty, its rows written a page
/// at a time: how many runs it listed.
fn realm_json(
    hearth: &Named<'_>,
    (read, filter): Reading<'_>,
    out: &mut impl Write,
) -> Result<usize> {
    let (realm, journal) = (
        serde_json::to_string(hearth.realm)?,
        serde_json::to_string(hearth.journal)?,
    );
    let head = format!(
        "    {{\n      \"realm\": {realm},\n      \"journal\": {journal},\n      \"runs\": ["
    );
    out.write_all(head.as_bytes())?;
    let reach = list_journal(&hearth.hearth.journal, read, |lister| match lister {
        Ok(lister) => lister.filtered(filter).json_rows(out, "        "),
        Err(refused) => Ok(Reach::refused(refused)),
    })?;
    let listed = reach.rows;
    close_array(out, listed, "      ")?;
    let detail = serde_json::to_string(&reach.detail())?;
    let tail = format!(",\n      \"count\": {listed},\n      \"detail\": {detail}\n    }}");
    out.write_all(tail.as_bytes())?;
    Ok(listed)
}

/// The table under each realm's heading, a hearth at a time: its columns
/// sized and its runs counted over every page of its hearth first, as the
/// one-hearth table's are, then the pages that read written, then how its
/// runs ended.
fn world_text(
    named: &[Named<'_>],
    (read, filter): Reading<'_>,
    style: &Style,
    out: &mut impl Write,
) -> Result<()> {
    for (index, hearth) in named.iter().enumerate() {
        let head = |reach: &Reach, tally: &Tally| {
            let names = (hearth.realm, hearth.journal);
            let detail = reach.detail();
            render::realm_head(index, names, tally, detail.as_deref(), style)
        };
        list_journal(&hearth.hearth.journal, read, |lister| match lister {
            Ok(lister) => {
                let lister = lister.filtered(filter);
                let (widths, tally, reach) = lister.widths()?;
                out.write_all(head(&reach, &tally).as_bytes())?;
                lister.text_rows(out, &widths, reach.pages, style)?;
                Ok(out.write_all(tally.closing().as_bytes())?)
            }
            Err(refused) => {
                let head = head(&Reach::refused(refused), &Tally::default());
                Ok(out.write_all(head.as_bytes())?)
            }
        })?;
    }
    Ok(())
}
