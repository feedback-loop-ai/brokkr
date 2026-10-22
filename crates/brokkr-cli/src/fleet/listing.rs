//! `brokkr runs` read and written a page at a time (decision 0089 ruling
//! 6), so its memory is a page of rows and the largest run's whole values
//! however many runs the hearth holds. With no flag it lists every run, in
//! the shape it always printed, the runs behind their summary past the
//! hundred a refresh reads each listed at its place, keyed a batch at a
//! time, and all of it one read of the journal; `--limit`
//! prints one page, `--after` the page after a cursor, and their JSON
//! states `next`.

use std::io::Write;
use std::path::Path;

use anyhow::Result;
use brokkr_store::{Store, StoreError};
use brokkr_view::{Cursor, Page, Paged, RunRow, RunsView, VIEW_VERSION};

use super::page::TAIL_KEYS;
use super::{Ask, Journal, Pages, PAGE_ROWS};
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
    Escape(String),
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
            PageRefused::Escape(value) => {
                write!(formatter, "'{value}' does not percent-decode to text")
            }
        }
    }
}

impl std::error::Error for PageRefused {}

/// A page's cursor, typed from its text.
pub(crate) fn cursor(text: &str) -> Result<Cursor, PageRefused> {
    text.parse().map_err(PageRefused::Cursor)
}

/// `list`, handed the lister of the hearth `store` opens, its journal file
/// `before` it was opened, at `now`: of every run when `every`, else of a
/// page. It is one read of the journal, so the runs behind their summary,
/// the pages and the runs keyed past those are one membership whatever a
/// writer catches up meanwhile, and each run lists once (ruling 6).
pub(crate) fn list_hearth<T>(
    store: &Store,
    journal: (Option<Journal>, &str),
    every: bool,
    list: impl FnOnce(&Lister<'_>) -> Result<T>,
) -> Result<T> {
    list_hearth_in(store, journal, every.then_some(TAIL_KEYS), list)
}

/// [`list_hearth`], of every run when `batched` names how many keys of
/// the runs behind past a refresh it holds at once, else of a page.
pub(crate) fn list_hearth_in<T>(
    store: &Store,
    (before, now): (Option<Journal>, &str),
    batched: Option<usize>,
    list: impl FnOnce(&Lister<'_>) -> Result<T>,
) -> Result<T> {
    store.as_one_read(|| {
        let pages = match batched {
            Some(keys) => Pages::every(store, before, keys),
            None => Pages::listing(store, before),
        };
        let pages = pages.map_err(refused)?;
        list(&Lister {
            pages: &pages,
            store,
            now,
        })
    })
}

/// `list`, handed the lister of every run of the hearth whose journal is
/// at `journal` at `now`, `keys` of the runs behind past a refresh keyed at
/// a time, in one read; or the store's words where the journal will not
/// open or its pages will not list: a many-hearth listing survives a
/// hearth it cannot read, as it survives one corrupt run. Opened
/// READ-ONLY: a reading surface creates no journal (decision 0026 ruling
/// 5).
pub(crate) fn list_journal<T>(
    journal: &Path,
    (now, keys): (&str, usize),
    list: impl FnOnce(Result<&Lister<'_>, String>) -> Result<T>,
) -> Result<T> {
    let before = Journal::at(journal);
    let store = match Store::open_read_only(journal) {
        Ok(store) => store,
        Err(error) => return list(Err(error.to_string())),
    };
    store.as_one_read(|| match Pages::every(&store, before, keys) {
        Ok(pages) => list(Ok(&Lister {
            pages: &pages,
            store: &store,
            now,
        })),
        Err(error) => list(Err(error.to_string())),
    })
}

/// What parts the JSON array item after `written` items from those
/// before it: a line break, after a comma past the first.
pub(crate) fn item_break(written: usize) -> &'static [u8] {
    if written == 0 {
        b"\n"
    } else {
        b",\n"
    }
}

/// The `]` that ends a JSON array of `count` items, on a line of its own
/// `indent`ed where it holds any.
pub(crate) fn close_array(out: &mut impl Write, count: usize, indent: &str) -> Result<()> {
    if count > 0 {
        write!(out, "\n{indent}")?;
    }
    Ok(out.write_all(b"]")?)
}

/// The store's refusal of a listing, in its own words: of its pages, or
/// of a page it reads.
fn refused(error: StoreError) -> anyhow::Error {
    anyhow::Error::msg(error.to_string())
}

/// One hearth's pages, as `brokkr runs` reads them: every value whole.
pub(crate) struct Lister<'a> {
    pages: &'a Pages,
    store: &'a Store,
    now: &'a str,
}

impl Lister<'_> {
    fn page(&self, after: Option<&Cursor>, limit: usize) -> Result<Page> {
        let ask = Ask {
            now: self.now,
            whole: &|_| true,
            keep: &|_| true,
        };
        self.pages
            .page(self.store, ask, after, limit)
            .map_err(refused)
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

    /// One page of `limit` rows after `after`, or the first, its `next`,
    /// and the runs behind their summary its refresh left unread: the
    /// view a page-asking reader is handed.
    pub(crate) fn paged(&self, after: Option<&Cursor>, limit: usize) -> Result<RunsView> {
        let (mut view, paged) = self.view(after, limit)?;
        view.page = Some(paged);
        Ok(view)
    }

    /// [`Self::paged`]'s rows, and its `next` and runs behind beside them.
    fn view(&self, after: Option<&Cursor>, limit: usize) -> Result<(RunsView, Paged)> {
        let page = self.page(after, limit)?;
        let view = RunsView::of(page.rows.into_iter().map(|(_, row)| row).collect());
        let paged = Paged {
            next: page.next,
            behind: self.pages.unread(),
        };
        Ok((view, paged))
    }

    /// Every run as `--json` prints it: the view model `brokkr runs` has
    /// always printed, pretty, written a page of rows at a time.
    pub(crate) fn every_json(&self, out: &mut impl Write) -> Result<()> {
        write!(
            out,
            "{{\n  \"view_version\": {VIEW_VERSION},\n  \"runs\": ["
        )?;
        let count = self.json_rows(out, "    ")?;
        close_array(out, count, "  ")?;
        writeln!(out, ",\n  \"count\": {count}\n}}")?;
        Ok(())
    }

    /// Every run's row, pretty, each of its lines `indent`ed, as the items
    /// of an open JSON array, written a page of rows at a time: how many.
    pub(crate) fn json_rows(&self, out: &mut impl Write, indent: &str) -> Result<usize> {
        let mut count = 0usize;
        self.each_page(|rows| {
            for row in rows {
                out.write_all(item_break(count))?;
                let text = serde_json::to_string_pretty(&row)?;
                let indented: Vec<String> =
                    text.lines().map(|line| format!("{indent}{line}")).collect();
                out.write_all(indented.join("\n").as_bytes())?;
                count += 1;
            }
            Ok(())
        })?;
        Ok(count)
    }

    /// Every run as the text table prints it: the columns sized over every
    /// page first, then each page written, so the pages line up as one.
    pub(crate) fn every_text(&self, out: &mut impl Write, style: &Style) -> Result<()> {
        let (widths, _) = self.widths()?;
        self.text_rows(out, &widths, style)
    }

    /// The table's columns sized over every page, and how many rows the
    /// pages hold.
    pub(crate) fn widths(&self) -> Result<([usize; 5], usize)> {
        let (mut widths, mut count) = ([0usize; 5], 0);
        self.each_page(|rows| {
            render::widen(&mut widths, &rows, self.now);
            count += rows.len();
            Ok(())
        })?;
        Ok((widths, count))
    }

    /// Every page's rows as the table prints them, each sized column
    /// `widths` wide.
    pub(crate) fn text_rows(
        &self,
        out: &mut impl Write,
        widths: &[usize; 5],
        style: &Style,
    ) -> Result<()> {
        self.each_page(|rows| {
            out.write_all(render::lines_within(&rows, widths, self.now, style).as_bytes())?;
            Ok(())
        })
    }

    /// The page after `after`, or the first, of `limit` rows: the view
    /// model with its `next`, or the table and the command that prints
    /// the next page, `called` as this one was.
    pub(crate) fn one(
        &self,
        out: &mut impl Write,
        (after, limit): (Option<&Cursor>, usize),
        json: bool,
        called: Called<'_>,
        style: &Style,
    ) -> Result<()> {
        if json {
            let view = self.paged(after, limit)?;
            return Ok(writeln!(out, "{}", serde_json::to_string_pretty(&view)?)?);
        }
        let (view, Paged { next, behind }) = self.view(after, limit)?;
        out.write_all(render::runs(&view, self.now, style).as_bytes())?;
        if let Some(line) = brokkr_view::behind_line(behind) {
            writeln!(out, "{line}")?;
        }
        if let Some(next) = next {
            writeln!(out, "{}", next_page(limit, &next, called))?;
        }
        Ok(())
    }
}

/// The command that prints the page after `next`, `called` as this
/// listing was, or, where a journal flag's path is not UTF-8, the flags
/// it takes in words: such a path is not printed as a command (#820).
fn next_page(limit: usize, next: &Cursor, called: Called<'_>) -> String {
    let after = render::shell_word(&next.to_string());
    match called.flags() {
        Some(called) => format!("next page  brokkr runs{called} --limit {limit} --after {after}"),
        None => format!(
            "next page  --limit {limit} --after {after}, with this listing's --db and --realms: \
             a path that is not UTF-8 is not printed as a command"
        ),
    }
}

/// The journal flags `brokkr runs` was called with, as typed: the next
/// page's command repeats them, so it reads the journal this page did
/// (#820).
#[derive(Clone, Copy, Default)]
pub(crate) struct Called<'a> {
    pub(crate) db: Option<&'a Path>,
    pub(crate) realms: Option<&'a Path>,
}

impl Called<'_> {
    /// ` --db <path>` and ` --realms <path>`, each given one, in that
    /// order, each path one shell word, joined to its flag by `=` where it
    /// starts with `-`, the one form clap reads such a value in; `None`
    /// where a path is not UTF-8, which no printed word would name as
    /// itself.
    fn flags(&self) -> Option<String> {
        let flags = [("--db", self.db), ("--realms", self.realms)];
        flags
            .into_iter()
            .filter_map(|(flag, path)| Some((flag, path?)))
            .map(|(flag, path)| {
                let path = path.to_str()?;
                let joint = if path.starts_with('-') { '=' } else { ' ' };
                Some(format!(" {flag}{joint}{}", render::shell_word(path)))
            })
            .collect()
    }
}
