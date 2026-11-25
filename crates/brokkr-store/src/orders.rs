//! The realm's append-only work-order journal (decision 0116 rulings 2, 5 and 10).

use std::num::NonZeroU32;

use brokkr_core::work_order::{
    text_digest, Trackers, WorkOrder, WorkOrderError, WorkOrderId, WorkOrderState,
};
use rusqlite::{params, Connection, OptionalExtension, Row, Transaction, TransactionBehavior};
use thiserror::Error;

use crate::schema::all_present;
use crate::{now_rfc3339, patiently, Attribution, Damage, Store, StoreError};

const ORDERS_SCHEMA_KEY: &str = "orders_schema";
const ORDERS_SCHEMA: u32 = 1;
const UPDATE_GUARD: &str = "work_order_events_append_only_update";
const DELETE_GUARD: &str = "work_order_events_append_only_delete";
const PROPOSED_INDEX: &str = "work_order_events_proposed";

const MIGRATION_ORDERS_V1: &str = r#"
CREATE TABLE IF NOT EXISTS work_order_events (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    number INTEGER NOT NULL CHECK (number > 0),
    event TEXT NOT NULL CHECK (event IN ('proposed', 'edited', 'ruled')),
    text TEXT,
    digest TEXT NOT NULL,
    operator TEXT NOT NULL,
    reason TEXT NOT NULL,
    recorded_at TEXT NOT NULL,
    CHECK ((event = 'ruled') = (text IS NULL))
);
CREATE UNIQUE INDEX IF NOT EXISTS work_order_events_proposed
    ON work_order_events (number) WHERE event = 'proposed';
CREATE TRIGGER IF NOT EXISTS work_order_events_append_only_update
    BEFORE UPDATE ON work_order_events
    BEGIN SELECT RAISE(ABORT, 'work order events are append-only'); END;
CREATE TRIGGER IF NOT EXISTS work_order_events_append_only_delete
    BEFORE DELETE ON work_order_events
    BEGIN SELECT RAISE(ABORT, 'work order events are append-only'); END;
"#;

/// A work-order read or operator command the journal refused.
#[derive(Debug, Error)]
pub enum OrderRefusal {
    #[error("this build does not read work order storage version {0}")]
    SchemaMismatch(u32),
    #[error("no work order was proposed as {0}")]
    Unknown(WorkOrderId),
    #[error("{0} was already proposed; edit it to change its text")]
    Exists(WorkOrderId),
    #[error("the edit names {0}; it must name the work order being edited or omit its id")]
    OtherId(WorkOrderId),
    #[error("{0} has an open ask; close its asks before ruling it")]
    OpenAsk(WorkOrderId),
    #[error("{0} is already ruled at its current text")]
    AlreadyRuled(WorkOrderId),
    #[error("the text of {0} changed since that digest was read; show it again and rule the digest it prints")]
    StaleDigest(WorkOrderId),
    #[error("the journal's text for {0} cannot be read: {1}")]
    Unreadable(WorkOrderId, #[source] WorkOrderError),
}

/// Read the version without taking a write lock. Leave versions this
/// binary does not know to their own binaries.
pub(crate) fn orders_intact(conn: &Connection) -> Result<bool, StoreError> {
    match orders_schema(conn)? {
        None => Ok(false),
        Some(ORDERS_SCHEMA) => Ok(all_present(conn, "trigger", &[UPDATE_GUARD, DELETE_GUARD])?
            && all_present(conn, "index", &[PROPOSED_INDEX])?),
        Some(_) => Ok(true),
    }
}

/// Install storage on a connection already holding the write lock.
pub(crate) fn migrate_orders(conn: &Connection) -> Result<(), StoreError> {
    const ORDERS_SCHEMA_INSERT: &str = "INSERT OR IGNORE INTO meta (key, value) VALUES (?1, ?2)";
    if orders_schema(conn)?.is_some_and(|found| found != ORDERS_SCHEMA) {
        return Ok(());
    }
    conn.execute_batch(MIGRATION_ORDERS_V1)?;
    let schema = ORDERS_SCHEMA.to_string();
    conn.execute(ORDERS_SCHEMA_INSERT, params![ORDERS_SCHEMA_KEY, schema])?;
    Ok(())
}

/// Read the recorded version and check that its table still holds the history.
pub(crate) fn orders_schema(conn: &Connection) -> Result<Option<u32>, StoreError> {
    let recorded = conn
        .query_row(
            "SELECT value FROM meta WHERE key = ?1",
            [ORDERS_SCHEMA_KEY],
            |row| Ok(row.get::<_, String>(0)),
        )
        .optional()?;
    let Some(recorded) = recorded else {
        return Ok(None);
    };
    let damaged = || StoreError::Damaged {
        journal: conn.path().unwrap_or_default().into(),
        damage: Damage::Orders,
    };
    let recorded = recorded.map_err(|_| damaged())?;
    let version = recorded.parse::<NonZeroU32>().map_err(|_| damaged())?;
    if !all_present(conn, "table", &["work_order_events"])? {
        return Err(damaged());
    }
    Ok(Some(version.get()))
}

fn orders_stored(conn: &Connection) -> Result<(), StoreError> {
    match orders_schema(conn)?.unwrap_or(0) {
        ORDERS_SCHEMA => Ok(()),
        found => Err(OrderRefusal::SchemaMismatch(found).into()),
    }
}

// A proposal establishes the number. A ruling must follow the latest text
// and name its digest, even when an edit repeats the previous text verbatim.
const CURRENT: &str = "
    SELECT proposed.number, latest.text, latest.digest, EXISTS (
        SELECT 1 FROM work_order_events AS ruling
        WHERE ruling.number = proposed.number AND ruling.event = 'ruled'
            AND ruling.seq > latest.seq AND ruling.digest = latest.digest
    )
    FROM work_order_events AS proposed
    JOIN work_order_events AS latest ON latest.seq = (
        SELECT MAX(text.seq) FROM work_order_events AS text
        WHERE text.number = proposed.number AND text.event IN ('proposed', 'edited')
    )
    WHERE proposed.event = 'proposed' AND (?1 IS NULL OR proposed.number = ?1)
    ORDER BY proposed.number";

struct Current {
    id: WorkOrderId,
    text: String,
    digest: String,
    ruled: bool,
}

impl Current {
    fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: read_id(row)?,
            text: row.get(1)?,
            digest: row.get(2)?,
            ruled: row.get(3)?,
        })
    }

    fn order(self, trackers: &Trackers) -> Result<WorkOrder, StoreError> {
        let state = if self.ruled {
            WorkOrderState::Ruled
        } else {
            WorkOrderState::Proposed
        };
        let order = WorkOrder::parse(&self.text, trackers)
            .map_err(|error| OrderRefusal::Unreadable(self.id, error))?;
        Ok(order.with_state(state))
    }
}

fn current(conn: &Connection, id: &WorkOrderId) -> Result<Current, StoreError> {
    conn.query_row(CURRENT, [id.number()], Current::read)
        .optional()?
        .ok_or_else(|| OrderRefusal::Unknown(id.clone()).into())
}

fn proposed(conn: &Connection, id: &WorkOrderId) -> Result<bool, StoreError> {
    const IS_PROPOSED: &str =
        "SELECT EXISTS (SELECT 1 FROM work_order_events WHERE number = ?1 AND event = 'proposed')";
    let proposed = conn.query_row(IS_PROPOSED, [id.number()], |row| row.get(0))?;
    Ok(proposed)
}

fn next_id(conn: &Connection) -> Result<WorkOrderId, StoreError> {
    const NEXT_ID: &str = "SELECT COALESCE(MAX(number), 0) + 1 FROM work_order_events";
    Ok(conn.query_row(NEXT_ID, [], read_id)?)
}

fn read_id(row: &Row<'_>) -> rusqlite::Result<WorkOrderId> {
    let number = row.get(0)?;
    WorkOrderId::from_number(number)
        .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(0, i64::from(number)))
}

enum Event<'a> {
    Proposed(&'a str),
    Edited(&'a str),
    Ruled(&'a str),
}

fn record(
    conn: &Connection,
    id: &WorkOrderId,
    event: Event<'_>,
    by: Attribution<'_>,
) -> Result<(), StoreError> {
    const INSERT_EVENT: &str =
        "INSERT INTO work_order_events (number, event, text, digest, operator, reason, recorded_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)";
    let (word, text, digest) = match event {
        Event::Proposed(text) => ("proposed", Some(text), text_digest(text)),
        Event::Edited(text) => ("edited", Some(text), text_digest(text)),
        Event::Ruled(digest) => ("ruled", None, digest.to_owned()),
    };
    let number = id.number();
    let recorded_at = now_rfc3339();
    let parameters = params![
        number,
        word,
        text,
        digest,
        by.operator,
        by.reason,
        recorded_at
    ];
    conn.execute(INSERT_EVENT, parameters)?;
    Ok(())
}

fn proposed_text(order: &WorkOrder, id: &WorkOrderId) -> String {
    order
        .clone()
        .with_id(id.clone())
        .with_state(WorkOrderState::Proposed)
        .render()
}

impl Store {
    /// Propose a new work order, preserving its supplied id or assigning the next one.
    pub fn order_add(
        &mut self,
        order: WorkOrder,
        by: Attribution<'_>,
    ) -> Result<WorkOrderId, StoreError> {
        self.write_order("order_add", |tx| {
            let id = if let Some(id) = order.id() {
                if proposed(tx, id)? {
                    return Err(OrderRefusal::Exists(id.clone()).into());
                }
                id.clone()
            } else {
                next_id(tx)?
            };
            let text = proposed_text(&order, &id);
            record(tx, &id, Event::Proposed(&text), by)?;
            Ok(id)
        })
    }

    /// Journal replacement text and return the work order to proposed.
    pub fn order_edit(
        &mut self,
        id: &WorkOrderId,
        order: WorkOrder,
        by: Attribution<'_>,
    ) -> Result<(), StoreError> {
        self.write_order("order_edit", |tx| {
            if !proposed(tx, id)? {
                return Err(OrderRefusal::Unknown(id.clone()).into());
            }
            if let Some(other) = order.id().filter(|other| *other != id) {
                return Err(OrderRefusal::OtherId(other.clone()).into());
            }
            let text = proposed_text(&order, id);
            record(tx, id, Event::Edited(&text), by)
        })
    }

    /// Record a ruling if the operator read the current digest and all asks are closed.
    pub fn order_rule(
        &mut self,
        id: &WorkOrderId,
        digest: &str,
        by: Attribution<'_>,
    ) -> Result<(), StoreError> {
        self.write_order("order_rule", |tx| {
            let latest = current(tx, id)?;
            if digest != latest.digest {
                return Err(OrderRefusal::StaleDigest(id.clone()).into());
            }
            if latest.ruled {
                return Err(OrderRefusal::AlreadyRuled(id.clone()).into());
            }
            let open = WorkOrder::journaled_open_asks(&latest.text)
                .map_err(|error| OrderRefusal::Unreadable(id.clone(), error))?;
            if open > 0 {
                return Err(OrderRefusal::OpenAsk(id.clone()).into());
            }
            record(tx, id, Event::Ruled(&latest.digest), by)
        })
    }

    /// Read every current work order in ascending number order.
    pub fn orders(&self, trackers: &Trackers) -> Result<Vec<WorkOrder>, StoreError> {
        self.read_orders("orders", |tx| {
            let mut statement = tx.prepare(CURRENT)?;
            let rows = statement.query_map([None::<u32>], Current::read)?;
            rows.map(|row| row?.order(trackers)).collect()
        })
    }

    /// Read the current text and journaled state of one work order.
    pub fn order(&self, id: &WorkOrderId, trackers: &Trackers) -> Result<WorkOrder, StoreError> {
        self.read_orders("order", |tx| current(tx, id)?.order(trackers))
    }

    fn write_order<T>(
        &mut self,
        operation: &'static str,
        mut write: impl FnMut(&Transaction<'_>) -> Result<T, StoreError>,
    ) -> Result<T, StoreError> {
        let conn = &mut self.conn;
        patiently(operation, self.patience, &self.path, || {
            let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
            orders_stored(&tx)?;
            let result = write(&tx)?;
            tx.commit()?;
            Ok(result)
        })
    }

    fn read_orders<T>(
        &self,
        operation: &'static str,
        mut read: impl FnMut(&Transaction<'_>) -> Result<T, StoreError>,
    ) -> Result<T, StoreError> {
        patiently(operation, self.patience, &self.path, || {
            let snapshot = self.conn.unchecked_transaction()?;
            orders_stored(&snapshot)?;
            read(&snapshot)
        })
    }
}

#[cfg(test)]
mod tests;
