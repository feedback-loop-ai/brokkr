//! `brokkr runs` read and written a page at a time (decision 0089 ruling
//! 6), so its memory is a page of rows and the largest run's whole values
//! however many runs the hearth holds. With no flag it lists every run, in
//! the shape it always printed; `--limit` prints one page, `--after` the
//! page after a cursor, and their JSON states `next`.

use std::io::Write;

use anyhow::Result;
use brokkr_store::Store;
use brokkr_view::{Cursor, Page, Paged, RunRow, RunsView, VIEW_VERSION};

use super::{Pages, PAGE_ROWS};
use crate::render::{self, Style};

/// Why an ask names no page, at the edge it was typed at: `--after`, or
/// `/api/runs`'s query (decision 0071 ruling 8). The view's refusal is
/// carried as clap carries an error: its crate names no `std::error`
/// (decision 0071 ruling 1).
#[derive(Debug)]
pub(crate) enum PageRefused {
    Cursor(brokkr_view::CursorError),
    Limit(String),
    Query(String),
}

impl std::fmt::Display for PageRefused {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PageRefused::Cursor(refused) => refused.fmt(formatter),
            PageRefused::Limit(limit) => {
                write!(formatter, "limit '{limit}' is not a number of runs from 1")
            }
            PageRefused::Query(pair) => {
                write!(formatter, "'{pair}' is neither limit=N nor after=CURSOR")
            }
        }
    }
}

impl std::error::Error for PageRefused {}

/// A page's cursor, typed from its text.
pub(crate) fn cursor(text: &str) -> Result<Cursor, PageRefused> {
    text.parse().map_err(PageRefused::Cursor)
}

/// One hearth's pages, as `brokkr runs` reads them: every value whole.
pub(crate) struct Lister<'a> {
    pub(crate) pages: &'a Pages,
    pub(crate) store: &'a Store,
    pub(crate) now: &'a str,
}

impl Lister<'_> {
    fn page(&self, after: Option<&Cursor>, limit: usize) -> Result<Page> {
        let page = self
            .pages
            .page(self.store, self.now, after, limit, &|_| true);
        page.map_err(|error| anyhow::Error::msg(error.to_string()))
    }

    /// Every page from the first, each handed to `each` in turn.
    fn each_page(&self, mut each: impl FnMut(Vec<RunRow>) -> Result<()>) -> Result<()> {
        let mut after = None;
        loop {
            let page = self.page(after.as_ref(), PAGE_ROWS)?;
            each(page.rows.into_iter().map(|(_, row)| row).collect())?;
            match page.next {
                Some(next) => after = Some(next),
                None => return Ok(()),
            }
        }
    }

    /// Every run, the sections in order, read a page at a time.
    pub(crate) fn rows(&self) -> Result<Vec<RunRow>> {
        let mut every = Vec::new();
        self.each_page(|rows| {
            every.extend(rows);
            Ok(())
        })?;
        Ok(every)
    }

    /// One page of `limit` rows after `after`, or the first, and its
    /// `next`: the view a page-asking reader is handed.
    pub(crate) fn paged(&self, after: Option<&Cursor>, limit: usize) -> Result<RunsView> {
        let page = self.page(after, limit)?;
        let mut view = RunsView::of(page.rows.into_iter().map(|(_, row)| row).collect());
        view.page = Some(Paged { next: page.next });
        Ok(view)
    }

    /// Every run as `--json` prints it: the view model `brokkr runs` has
    /// always printed, pretty, written a page of rows at a time.
    pub(crate) fn every_json(&self, out: &mut impl Write) -> Result<()> {
        write!(
            out,
            "{{\n  \"view_version\": {VIEW_VERSION},\n  \"runs\": ["
        )?;
        let mut count = 0usize;
        self.each_page(|rows| {
            for row in rows {
                out.write_all(if count == 0 { b"\n" } else { b",\n" })?;
                let text = serde_json::to_string_pretty(&row)?;
                let indented: Vec<String> =
                    text.lines().map(|line| format!("    {line}")).collect();
                out.write_all(indented.join("\n").as_bytes())?;
                count += 1;
            }
            Ok(())
        })?;
        if count > 0 {
            out.write_all(b"\n  ")?;
        }
        writeln!(out, "],\n  \"count\": {count}\n}}")?;
        Ok(())
    }

    /// Every run as the text table prints it: the columns sized over every
    /// page first, then each page written, so the pages line up as one.
    pub(crate) fn every_text(&self, out: &mut impl Write, style: &Style) -> Result<()> {
        let mut widths = [0usize; 5];
        self.each_page(|rows| {
            render::widen(&mut widths, &rows, self.now);
            Ok(())
        })?;
        self.each_page(|rows| {
            out.write_all(render::lines_within(&rows, &widths, self.now, style).as_bytes())?;
            Ok(())
        })
    }

    /// The page after `after`, or the first, of `limit` rows: the view
    /// model with its `next`, or the table and the command that prints
    /// the next page.
    pub(crate) fn one(
        &self,
        out: &mut impl Write,
        (after, limit): (Option<&Cursor>, usize),
        json: bool,
        style: &Style,
    ) -> Result<()> {
        let view = self.paged(after, limit)?;
        if json {
            return Ok(writeln!(out, "{}", serde_json::to_string_pretty(&view)?)?);
        }
        out.write_all(render::runs(&view, self.now, style).as_bytes())?;
        if let Some(Paged { next: Some(next) }) = &view.page {
            writeln!(out, "next page  brokkr runs --limit {limit} --after {next}")?;
        }
        Ok(())
    }
}
