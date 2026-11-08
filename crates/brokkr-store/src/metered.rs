//! The aggregate readouts' window reads of spends and sites (decision 0089
//! ruling 7, slice 4's unit U3b): every reported cost whose checkpoint falls
//! in the window, and every site that ran in it, found through the indexes
//! #1028 built and streamed one row at a time.
//!
//! A spend is one `event_index` checkpoint row in `[from, to)` carrying a
//! `usd`, read by the `(at_ms)` partial index, and its run's realm key from
//! `run_summary`. A site ran in the window when it started there, through
//! `seat_summary_started`, or when one of its checkpoints fell there: the
//! checkpoint index's range, each checkpoint's site looked up through
//! `seat_summary_site` as `seat_read`'s `site_of_checkpoint` looks it up,
//! and the site's row joined by its key. The started set and the
//! metered set are one `UNION ALL`, each site's in-window checkpoints beside
//! it, sorted once by the figure's cut keys so the caller folds one site at
//! a time and holds nothing of the window's other sites.
//!
//! A run whose summary row sits behind its head, or whose seat rows or
//! checkpoint facts stand behind the head the row names, is in neither
//! read: its rows are older than the head it stands at, so the caller reads
//! that run whole instead (decision 0089 ruling 8).
//! [`Store::window_fallbacks`] names the projection lag the summary lane
//! does not, and U4 combines the reads with the whole read, so a lagging
//! run seen here as well would count its sites and costs twice.
//!
//! A site's in-window facts are folded while the stream passes them, never
//! collected: `reported` is whether any in-window checkpoint reported a
//! cost, and `usage` is brokkr-core's [`counted`] over each in-window
//! checkpoint's stored `turn`, `session` and tokens, in Rust ("SQL decides
//! nothing", #1009). A turn whose usage repeats its site's last message
//! adds none, as the whole-run reading's `meter::once` recorded on its row.
//! A site's rule is its row's `metering`, decided over its whole run and
//! never over the window, so a window holding only some of its checkpoints
//! never reads both a turn and the session record that settles it.
//!
//! The reads have no production caller until slice 4's unit U4 lands them
//! in `brokkr-cli`; they are `pub` on [`Store`] so it can, and each is in
//! the public API snapshot until then.

use std::sync::LazyLock;

use brokkr_core::page::Lane;
use brokkr_core::seat::meter::{counted, sum_usage, Metering};
use brokkr_core::seat::{finish_usage, TokenUsage};
use rusqlite::types::{Value, ValueRef};
use rusqlite::{params_from_iter, Row, StatementStatus};

use crate::event_index::CHECKPOINTED;
use crate::{Store, StoreError};

/// The order a window's sites are folded in (decision 0089 ruling 7): the
/// four key orders the figures group by. Each is the stored cut key, never
/// the whole value: the writer cut it where it kept it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SiteOrder {
    /// By the stored model key, then the run and the site.
    Model,
    /// By the stored provider key, then the run and the site.
    Provider,
    /// By the stored realm key and then the provider key.
    RealmProvider,
    /// By the stored realm key, the recipe key and then the provider key.
    RecipeProvider,
}

/// One reported cost a window reads, borrowed from its row: the run it
/// belongs to, the realm the run named (NULL where it named none, so the
/// caller falls back to its hearth's label as `analytics`' facts does), the
/// checkpoint's time and phase, and the cost it reported.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowSpend<'a> {
    pub run_id: &'a str,
    pub realm: Option<&'a str>,
    pub at_ms: i64,
    pub phase: Option<&'a str>,
    pub usd: f64,
}

/// One run a window leaves whole, borrowed from its summary row: a run
/// whose seat rows or checkpoint facts stand behind the head it names, so
/// the caller reads it whole and its sites and spends are counted once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowFallback<'a> {
    pub run_id: &'a str,
}

/// One site a window reads, borrowed from its row or held for the one site
/// being folded: its run and opened key, its run's aggregate columns, its
/// own dated facts, and what its in-window checkpoints reported and
/// metered.
#[derive(Debug, Clone)]
pub struct WindowSite<'a> {
    pub run_id: &'a str,
    /// The site's row key in its run's seat rows.
    pub opened: u64,
    /// The realm its run named, NULL where it named none: the caller
    /// falls back to the hearth's label as `analytics`' facts does.
    pub realm: Option<&'a str>,
    pub recipe: Option<&'a str>,
    /// Still running, parked or stopping, never concluded.
    pub open: bool,
    pub finished_ms: Option<i64>,
    pub phase: Option<&'a str>,
    pub provider: Option<&'a str>,
    pub model: Option<&'a str>,
    pub started_ms: Option<i64>,
    /// How the site's tokens are metered, decided over all its checkpoints.
    pub metering: Metering,
    /// Whether any in-window checkpoint reported a cost.
    pub reported: bool,
    /// What the in-window checkpoints metered by the site's rule, summed.
    pub usage: Option<TokenUsage>,
}

impl Store {
    /// The costs a window reads (ruling 7): every checkpoint row in
    /// `[from, to)` that carries a `usd`, through the checkpoint index, its
    /// run's realm key joined by the run's key. A checkpoint no site names
    /// reports here as it does whole: its cost counts. A run behind its
    /// summary row is left whole to the caller (ruling 8). `realm` narrows
    /// to one stored realm key where given.
    pub fn window_spends(
        &self,
        from: i64,
        to: i64,
        realm: Option<&str>,
        visit: impl FnMut(&WindowSpend<'_>),
    ) -> Result<(), StoreError> {
        self.window_spends_with(from, to, realm, visit, |_| {})
    }

    /// [`Store::window_spends`] with the statement's VM steps handed to
    /// `stepped`, so the test that binds the read's cost measures the
    /// production statement itself.
    fn window_spends_with(
        &self,
        from: i64,
        to: i64,
        realm: Option<&str>,
        mut visit: impl FnMut(&WindowSpend<'_>),
        mut stepped: impl FnMut(i64),
    ) -> Result<(), StoreError> {
        let mut statement = self.conn.prepare(&SPENDS)?;
        let bounds = bounds(from, to, realm);
        let mut rows = statement.query(params_from_iter(bounds))?;
        while let Some(row) = rows.next()? {
            visit(&WindowSpend {
                run_id: required_text(row, 0)?,
                realm: optional_text(row, 1)?,
                at_ms: row.get(2)?,
                phase: optional_text(row, 3)?,
                usd: row.get(4)?,
            });
        }
        drop(rows);
        stepped(i64::from(statement.get_status(StatementStatus::VmStep)));
        Ok(())
    }

    /// The sites a window reads (ruling 7): every site that started in
    /// `[from, to)`, through the started index, or that has a checkpoint
    /// in it, through the checkpoint index and its site's key. Each site is
    /// yielded once, in `order`, with its in-window facts folded as the
    /// stream passes them. A run behind its summary row is left whole to
    /// the caller (ruling 8). `realm` narrows to one stored realm key where
    /// given.
    pub fn window_sites(
        &self,
        from: i64,
        to: i64,
        realm: Option<&str>,
        order: SiteOrder,
        visit: impl FnMut(&WindowSite<'_>),
    ) -> Result<(), StoreError> {
        self.window_sites_with(from, to, realm, order, visit, |_| {})
    }

    /// [`Store::window_sites`] with the statement's VM steps handed to
    /// `stepped`, so the test that binds the read's cost measures the
    /// production statement itself.
    fn window_sites_with(
        &self,
        from: i64,
        to: i64,
        realm: Option<&str>,
        order: SiteOrder,
        mut visit: impl FnMut(&WindowSite<'_>),
        mut stepped: impl FnMut(i64),
    ) -> Result<(), StoreError> {
        let mut statement = self.conn.prepare(sites_sql(order))?;
        let bounds = bounds(from, to, realm);
        let mut rows = statement.query(params_from_iter(bounds))?;
        let mut folded = Folding::default();
        let mut metering = Metering::Session;
        while let Some(row) = rows.next()? {
            let key = (
                required_text(row, 0)?.to_string(),
                row.get::<_, i64>(1)? as u64,
            );
            if folded.key.as_ref() != Some(&key) {
                if let Some(held) = folded.held.take() {
                    folded.emit(&held, &mut visit);
                }
                metering = folded.begin(key, row)?;
            }
            if row.get::<_, i64>(11)? == 0 {
                folded.checkpoint(metering, row)?;
            }
        }
        drop(rows);
        if let Some(held) = folded.held.take() {
            folded.emit(&held, &mut visit);
        }
        stepped(i64::from(statement.get_status(StatementStatus::VmStep)));
        Ok(())
    }

    /// The runs a window leaves to the whole-run fallback (ruling 8): every
    /// run the window would read whose seat rows or checkpoint facts stand
    /// behind its summary head. The `behind` lane is `aggregate`'s
    /// `behind_runs`; this names the projection lag that lane does not, so
    /// a caller reads each run here whole and its sites and spends are
    /// counted once. `realm` narrows to one stored realm key where given.
    pub fn window_fallbacks(
        &self,
        from: i64,
        to: i64,
        realm: Option<&str>,
        mut visit: impl FnMut(&WindowFallback<'_>),
    ) -> Result<(), StoreError> {
        let mut statement = self.conn.prepare(&FALLBACKS)?;
        let bounds = bounds(from, to, realm);
        let mut rows = statement.query(params_from_iter(bounds))?;
        while let Some(row) = rows.next()? {
            visit(&WindowFallback {
                run_id: required_text(row, 0)?,
            });
        }
        Ok(())
    }

    /// The VM steps [`Store::window_spends`] takes, for the test that holds
    /// the range to the window alone.
    #[cfg(test)]
    pub(crate) fn window_spends_steps(
        &self,
        from: i64,
        to: i64,
        realm: Option<&str>,
    ) -> Result<i64, StoreError> {
        let mut steps = 0;
        self.window_spends_with(from, to, realm, |_| {}, |taken| steps = taken)?;
        Ok(steps)
    }

    /// The VM steps [`Store::window_sites`] takes, for the test that holds
    /// the range to the window alone.
    #[cfg(test)]
    pub(crate) fn window_sites_steps(
        &self,
        from: i64,
        to: i64,
        realm: Option<&str>,
        order: SiteOrder,
    ) -> Result<i64, StoreError> {
        let mut steps = 0;
        self.window_sites_with(from, to, realm, order, |_| {}, |taken| steps = taken)?;
        Ok(steps)
    }
}

/// The three values every read binds: the window and the realm key or NULL
/// for every realm.
fn bounds(from: i64, to: i64, realm: Option<&str>) -> [Value; 3] {
    [
        Value::Integer(from),
        Value::Integer(to),
        realm.map_or(Value::Null, |realm| Value::Text(realm.to_string())),
    ]
}

/// The lanes a run is still open in (ruling 7): a stopping run is a
/// running one with an accepted stop, still open.
fn open_lanes() -> String {
    [Lane::Running, Lane::Parked, Lane::Stopping]
        .map(|lane| format!("'{}'", lane.as_str()))
        .join(", ")
}

/// Whether `run`'s seat rows or checkpoint facts stand behind the summary
/// head it names (ruling 8): a head at another head, seat rows under no
/// head, or a checkpoint row still waiting for its facts. A window read
/// leaves such a run whole, so it never trusts a stale row, and
/// [`Store::window_fallbacks`] names it for the caller's whole read. A run
/// with no seat rows at all has no site to lose and is not a lag.
fn lags(run: &str, head: &str) -> String {
    format!(
        "(({head}.run_id IS NOT NULL
            AND ({head}.folded_seq <> {run}.head_seq OR {head}.folded_hash <> {run}.head_hash))
          OR ({head}.run_id IS NULL
              AND EXISTS (SELECT 1 FROM seat_summary WHERE run_id = {run}.run_id))
          OR EXISTS (SELECT 1 FROM event_index INDEXED BY event_index_checkpoint_stale
                      WHERE type = '{CHECKPOINTED}'
                        AND (facts IS NULL OR turn IS NULL OR repeat IS NULL OR facts_seq IS NOT seq)
                        AND run_id = {run}.run_id))"
    )
}

/// The spends' one statement: the checkpoint index's range, each row's run
/// joined by its key, a row no cost reported left out, and a run behind its
/// summary row or its projections left whole (ruling 8).
static SPENDS: LazyLock<String> = LazyLock::new(|| {
    format!(
        "SELECT e.run_id, r.realm, e.at_ms, e.phase, e.usd
    FROM event_index AS e INDEXED BY event_index_checkpoint
    JOIN run_summary AS r ON r.run_id = e.run_id
    LEFT JOIN seat_head AS h ON h.run_id = r.run_id
   WHERE e.type = '{CHECKPOINTED}' AND e.at_ms >= ?1 AND e.at_ms < ?2
     AND e.usd IS NOT NULL AND r.lane <> '{behind}' AND NOT {lags}
     AND (?3 IS NULL OR r.realm = ?3)",
        behind = Lane::Behind.as_str(),
        lags = lags("r", "h"),
    )
});

/// The runs a window leaves whole: those it would read whose projections
/// lag their summary head, each once, so a caller reads them whole.
static FALLBACKS: LazyLock<String> = LazyLock::new(|| {
    format!(
        "SELECT r.run_id
           FROM run_summary AS r LEFT JOIN seat_head AS h ON h.run_id = r.run_id
          WHERE r.finished_ms >= ?1 AND r.finished_ms < ?2
            AND r.lane <> '{behind}' AND {lags}
            AND (?3 IS NULL OR r.realm = ?3)
         UNION
         SELECT r.run_id
           FROM run_summary AS r LEFT JOIN seat_head AS h ON h.run_id = r.run_id
          WHERE r.lane IN ({open}) AND {lags}
            AND (?3 IS NULL OR r.realm = ?3)
          ORDER BY 1",
        behind = Lane::Behind.as_str(),
        open = open_lanes(),
        lags = lags("r", "h"),
    )
});

/// The sites' statements, one per order: the started set and the metered
/// set as one `UNION ALL`, each site's in-window checkpoints beside it,
/// then one sort by the order's cut keys. The site key and the marker tie
/// each site's checkpoint rows before the row its start contributed, so a
/// site that both started and metered in the window is folded once and
/// yielded once.
static SITES_MODEL: LazyLock<String> =
    LazyLock::new(|| sites_statement("ORDER BY model_key, run_id, opened, header, at_ms, seq"));
static SITES_PROVIDER: LazyLock<String> =
    LazyLock::new(|| sites_statement("ORDER BY provider_key, run_id, opened, header, at_ms, seq"));
static SITES_REALM_PROVIDER: LazyLock<String> = LazyLock::new(|| {
    sites_statement("ORDER BY realm, provider_key, run_id, opened, header, at_ms, seq")
});
static SITES_RECIPE_PROVIDER: LazyLock<String> = LazyLock::new(|| {
    sites_statement("ORDER BY realm, recipe, provider_key, run_id, opened, header, at_ms, seq")
});

/// The statement an order folds.
fn sites_sql(order: SiteOrder) -> &'static str {
    match order {
        SiteOrder::Model => &SITES_MODEL,
        SiteOrder::Provider => &SITES_PROVIDER,
        SiteOrder::RealmProvider => &SITES_REALM_PROVIDER,
        SiteOrder::RecipeProvider => &SITES_RECIPE_PROVIDER,
    }
}

/// The sites' one statement, `order` over its output column names.
///
/// The metered arm reads the checkpoint index's one range and finds each
/// checkpoint's site through the site index, the greatest start below its
/// seq, exactly as `site_of_checkpoint` does; the site's row is joined by
/// its key. The started arm reads the started index's range. A run behind
/// its summary row is in neither arm (ruling 8): the caller reads it whole.
/// A run's open lane is the one the fleet lists (a stopping run is still
/// open); the realm filter is on the run's stored key, as the run reads'
/// is. A run whose seat rows or checkpoint facts lag its summary head is in
/// neither arm (ruling 8): the caller reads it whole.
fn sites_statement(order: &str) -> String {
    format!(
        "WITH metered AS (
            SELECT e.run_id AS run_id, e.seq AS seq, e.at_ms AS at_ms,
                   e.reported AS reported, e.turn AS turn, e.session AS session,
                   e.repeat AS repeat,
                   e.input_tokens AS input_tokens, e.output_tokens AS output_tokens,
                   e.cache_read_tokens AS cache_read_tokens,
                   e.cache_write_tokens AS cache_write_tokens,
                   e.reasoning_output_tokens AS reasoning_output_tokens,
                   (SELECT s2.opened FROM seat_summary AS s2 INDEXED BY seat_summary_site
                     WHERE s2.run_id = e.run_id AND s2.attempt_id IS e.attempt_id
                       AND s2.member IS e.member AND s2.started_seq < e.seq
                       AND s2.site IS NOT NULL
                     ORDER BY s2.started_seq DESC LIMIT 1) AS opened
              FROM event_index AS e INDEXED BY event_index_checkpoint
             WHERE e.type = '{CHECKPOINTED}'
               AND e.at_ms >= ?1 AND e.at_ms < ?2
        )
        SELECT s.run_id AS run_id, s.opened AS opened, r.realm AS realm,
               r.recipe AS recipe,
               (r.lane IN ({open})) AS open,
               r.finished_ms AS finished_ms,
               s.phase_key AS phase_key, s.provider_key AS provider_key,
               s.model_key AS model_key, s.started_ms AS started_ms,
               s.metering AS metering,
               0 AS header, w.at_ms AS at_ms, w.seq AS seq,
               w.reported AS reported, w.turn AS turn, w.session AS session,
               w.repeat AS repeat, w.input_tokens AS input_tokens,
               w.output_tokens AS output_tokens,
               w.cache_read_tokens AS cache_read_tokens,
               w.cache_write_tokens AS cache_write_tokens,
               w.reasoning_output_tokens AS reasoning_output_tokens
          FROM metered AS w
          JOIN run_summary AS r ON r.run_id = w.run_id
          JOIN seat_summary AS s ON s.run_id = w.run_id AND s.opened = w.opened
          LEFT JOIN seat_head AS h ON h.run_id = r.run_id
         WHERE r.lane <> '{behind}' AND NOT {lags}
           AND (?3 IS NULL OR r.realm = ?3)
        UNION ALL
        SELECT s.run_id, s.opened, r.realm, r.recipe,
               (r.lane IN ({open})), r.finished_ms,
               s.phase_key, s.provider_key, s.model_key, s.started_ms, s.metering,
               1, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL
          FROM seat_summary AS s INDEXED BY seat_summary_started
          JOIN run_summary AS r ON r.run_id = s.run_id
          LEFT JOIN seat_head AS h ON h.run_id = r.run_id
         WHERE s.site IS NOT NULL AND s.started_ms >= ?1 AND s.started_ms < ?2
           AND r.lane <> '{behind}' AND NOT {lags}
           AND (?3 IS NULL OR r.realm = ?3)
        {order}",
        open = open_lanes(),
        behind = Lane::Behind.as_str(),
        lags = lags("r", "h"),
    )
}

/// One site's columns, held while its in-window checkpoints are folded: the
/// row's borrowed values made owned once a site, so the closure's one
/// yielded row borrows them.
struct Held {
    run_id: String,
    opened: u64,
    realm: Option<String>,
    recipe: Option<String>,
    open: bool,
    finished_ms: Option<i64>,
    phase: Option<String>,
    provider: Option<String>,
    model: Option<String>,
    started_ms: Option<i64>,
    metering: Metering,
}

/// What one site's in-window checkpoints have folded so far.
#[derive(Default)]
struct Facts {
    reported: bool,
    usage: Option<TokenUsage>,
}

/// The site the stream is folding, its held columns and its in-window
/// facts. One site at a time: a change of key yields the previous site and
/// starts the next, so nothing of the window's other sites is held.
#[derive(Default)]
struct Folding {
    key: Option<(String, u64)>,
    held: Option<Held>,
    facts: Facts,
}

impl Folding {
    /// Start folding the site `key`, whose columns are on `row`, and give
    /// back the rule its in-window checkpoints count by.
    fn begin(&mut self, key: (String, u64), row: &Row<'_>) -> Result<Metering, StoreError> {
        self.key = Some(key);
        self.facts = Facts::default();
        self.held = Some(Held {
            run_id: required_text(row, 0)?.to_string(),
            opened: row.get::<_, i64>(1)? as u64,
            realm: optional_text(row, 2)?.map(str::to_string),
            recipe: optional_text(row, 3)?.map(str::to_string),
            open: row.get::<_, i64>(4)? != 0,
            finished_ms: row.get(5)?,
            phase: optional_text(row, 6)?.map(str::to_string),
            provider: optional_text(row, 7)?.map(str::to_string),
            model: optional_text(row, 8)?.map(str::to_string),
            started_ms: row.get(9)?,
            metering: metering(row, 10)?,
        });
        Ok(self
            .held
            .as_ref()
            .expect("begin sets the site it folds")
            .metering)
    }

    /// Fold one in-window checkpoint of the current site: whether it
    /// reported a cost, and what its tokens add under the site's rule. A
    /// turn whose usage repeats the site's last message adds none, as
    /// `meter::once` already decided on its row.
    fn checkpoint(&mut self, metering: Metering, row: &Row<'_>) -> Result<(), StoreError> {
        self.facts.reported |= row.get::<_, Option<i64>>(14)?.is_some_and(|set| set != 0);
        let turn = row.get::<_, Option<i64>>(15)?.is_some_and(|set| set != 0);
        let session = row.get::<_, Option<i64>>(16)?.is_some_and(|set| set != 0);
        let repeat = row.get::<_, Option<i64>>(17)?.is_some_and(|set| set != 0);
        let usage = usage(row, 18)?;
        let counted = counted(metering, turn, session, usage.filter(|_| !repeat));
        self.facts.usage = sum_usage(self.facts.usage.take().into_iter().chain(counted));
        Ok(())
    }

    /// Yield the site `held`, its facts folded to the end of its group.
    fn emit(&mut self, held: &Held, visit: &mut impl FnMut(&WindowSite<'_>)) {
        visit(&WindowSite {
            run_id: &held.run_id,
            opened: held.opened,
            realm: held.realm.as_deref(),
            recipe: held.recipe.as_deref(),
            open: held.open,
            finished_ms: held.finished_ms,
            phase: held.phase.as_deref(),
            provider: held.provider.as_deref(),
            model: held.model.as_deref(),
            started_ms: held.started_ms,
            metering: held.metering,
            reported: self.facts.reported,
            usage: self.facts.usage.clone(),
        });
    }
}

/// The metering rule a row keeps, or SQLite's refusal where it holds none
/// a writer words: a stored rule no reader may guess (ruling 8).
fn metering(row: &Row<'_>, at: usize) -> Result<Metering, StoreError> {
    let index: i64 = row.get(at)?;
    Metering::at(index).ok_or_else(|| rusqlite::Error::IntegralValueOutOfRange(at, index).into())
}

/// The tokens a checkpoint row carries, read as brokkr-core reads them: the
/// five parts and the total it makes again, none where it states none.
fn usage(row: &Row<'_>, at: usize) -> Result<Option<TokenUsage>, StoreError> {
    Ok(finish_usage(TokenUsage {
        input_tokens: row.get(at)?,
        output_tokens: row.get(at + 1)?,
        cache_read_tokens: row.get(at + 2)?,
        cache_write_tokens: row.get(at + 3)?,
        reasoning_output_tokens: row.get(at + 4)?,
        total_tokens: None,
    }))
}

/// The text at `at`, which every row's key names.
fn required_text<'a>(row: &'a Row<'_>, at: usize) -> Result<&'a str, rusqlite::Error> {
    Ok(row.get_ref(at)?.as_str()?)
}

/// The text at `at`, NULL where the column holds none.
fn optional_text<'a>(row: &'a Row<'_>, at: usize) -> Result<Option<&'a str>, rusqlite::Error> {
    match row.get_ref(at)? {
        ValueRef::Null => Ok(None),
        value => Ok(Some(value.as_str()?)),
    }
}

#[cfg(test)]
mod tests;
