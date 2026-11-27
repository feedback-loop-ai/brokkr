//! Stateful in-process PostgreSQL v3 wire-protocol mock server for offline tests.

use std::collections::{BTreeMap, BTreeSet};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::wire_tests::{
    data_row_payload, error_payload, read_client_frame, read_startup, send_frame,
};
use crate::PgConfig;

pub(crate) type QueueRow = (
    String,
    i64,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
);
pub(crate) type CooldownDbRow = (
    String,
    i64,
    String,
    String,
    String,
    i64,
    String,
    Option<i64>,
    i64,
    i64,
);
pub(crate) type EventDbRow = (String, String, String, String, String);
pub(crate) type ProjEventRow = (
    String,
    String,
    Option<String>,
    Option<String>,
    String,
    String,
    String,
);

#[derive(Clone, Default)]
pub(crate) struct MockDb {
    pub(crate) runs: BTreeMap<String, (String, String, String)>,
    pub(crate) events: BTreeMap<(String, i64), EventDbRow>,
    pub(crate) proj_runs: BTreeMap<String, (String, String, String, i64, String)>,
    pub(crate) proj_events: BTreeMap<(String, i64), ProjEventRow>,
    pub(crate) queue_next_id: i64,
    pub(crate) queue_entries: BTreeMap<i64, QueueRow>,
    pub(crate) queue_waits: BTreeMap<(i64, i64), String>,
    pub(crate) cooldown_next_seq: i64,
    pub(crate) cooldowns: BTreeMap<i64, CooldownDbRow>,
    pub(crate) locked_queue_ids: BTreeSet<i64>,
    pub(crate) queries: Vec<String>,
    pub(crate) force_next_error: Option<(String, String)>,
    pub(crate) force_insert_event_error: Option<(String, String)>,
    pub(crate) force_empty_returning: bool,
    pub(crate) force_zero_cooldown_insert: bool,
}

pub(crate) struct MockPgServer {
    pub(crate) port: u16,
    pub(crate) db: Arc<Mutex<MockDb>>,
    stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl MockPgServer {
    pub(crate) fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock pg");
        listener.set_nonblocking(true).expect("nonblocking");
        let port = listener.local_addr().unwrap().port();
        let db = Arc::new(Mutex::new(MockDb::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let db_bg = Arc::clone(&db);
        let stop_bg = Arc::clone(&stop);
        let handle = thread::spawn(move || {
            let mut conns = Vec::new();
            while !stop_bg.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        stream.set_nonblocking(false).expect("blocking conn");
                        let db_c = Arc::clone(&db_bg);
                        conns.push(thread::spawn(move || serve_conn(stream, db_c)));
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(std::time::Duration::from_millis(2));
                    }
                    Err(_) => break,
                }
            }
            for c in conns {
                let _ = c.join();
            }
        });
        Self {
            port,
            db,
            stop,
            handle: Some(handle),
        }
    }

    pub(crate) fn config(&self) -> PgConfig {
        PgConfig::parse(&format!(
            "postgres://brokkr:pw@127.0.0.1:{}/brokkr",
            self.port
        ))
        .unwrap()
    }
}

impl Drop for MockPgServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = TcpStream::connect(("127.0.0.1", self.port));
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

fn serve_conn(mut stream: TcpStream, db: Arc<Mutex<MockDb>>) {
    let _ = read_startup(&mut stream);
    send_frame(&mut stream, b'R', &0i32.to_be_bytes());
    send_frame(&mut stream, b'Z', b"I");

    let mut pending_sql = String::new();
    let mut pending_params: Vec<Option<String>> = Vec::new();
    let mut tx_snapshot: Option<MockDb> = None;

    while let Some((tag, body)) = read_client_frame(&mut stream) {
        match tag {
            b'X' => break,
            b'P' => {
                let rest = &body[1..];
                let end = rest.iter().position(|&b| b == 0).unwrap_or(rest.len());
                pending_sql = String::from_utf8_lossy(&rest[..end]).into_owned();
            }
            b'B' => pending_params = decode_bind_params(&body),
            b'D' | b'E' => {}
            b'S' => {
                let res = exec_sql(&db, &mut tx_snapshot, &pending_sql, &pending_params);
                send_frame(&mut stream, b'1', b"");
                send_frame(&mut stream, b'2', b"");
                match res {
                    Ok((rows, cmd)) => {
                        for row in &rows {
                            let refs: Vec<Option<&str>> =
                                row.iter().map(|c| c.as_deref()).collect();
                            send_frame(&mut stream, b'D', &data_row_payload(&refs));
                        }
                        let mut ctag = cmd.into_bytes();
                        ctag.push(0);
                        send_frame(&mut stream, b'C', &ctag);
                        send_frame(
                            &mut stream,
                            b'Z',
                            if tx_snapshot.is_some() { b"T" } else { b"I" },
                        );
                    }
                    Err((code, msg)) => {
                        send_frame(&mut stream, b'E', &error_payload(&code, &msg, None));
                        send_frame(
                            &mut stream,
                            b'Z',
                            if tx_snapshot.is_some() { b"E" } else { b"I" },
                        );
                    }
                }
            }
            _ => {}
        }
    }
}

fn decode_bind_params(body: &[u8]) -> Vec<Option<String>> {
    let mut off = 2;
    let fmt_count = i16::from_be_bytes([body[off], body[off + 1]]) as usize;
    off += 2 + fmt_count * 2;
    let count = i16::from_be_bytes([body[off], body[off + 1]]) as usize;
    off += 2;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let len = i32::from_be_bytes([body[off], body[off + 1], body[off + 2], body[off + 3]]);
        off += 4;
        if len == -1 {
            out.push(None);
        } else {
            let end = off + len as usize;
            out.push(Some(String::from_utf8_lossy(&body[off..end]).into_owned()));
            off = end;
        }
    }
    out
}

type SqlRes = Result<(Vec<Vec<Option<String>>>, String), (String, String)>;

fn pstr(params: &[Option<String>], idx: usize) -> &str {
    params[idx].as_deref().unwrap_or("")
}

fn pi64(params: &[Option<String>], idx: usize) -> i64 {
    pstr(params, idx).parse().unwrap()
}

fn exec_sql(
    db_arc: &Arc<Mutex<MockDb>>,
    tx_snapshot: &mut Option<MockDb>,
    sql: &str,
    params: &[Option<String>],
) -> SqlRes {
    let norm: String = sql.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut db = db_arc.lock().unwrap();
    db.queries.push(norm.clone());

    if norm == "BEGIN" {
        *tx_snapshot = Some(db.clone());
        return Ok((vec![], "BEGIN".into()));
    }
    if norm == "COMMIT" {
        *tx_snapshot = None;
        return Ok((vec![], "COMMIT".into()));
    }
    if norm == "ROLLBACK" {
        if let Some(snap) = tx_snapshot.take() {
            let queries = std::mem::take(&mut db.queries);
            *db = snap;
            db.queries = queries;
            db.force_empty_returning = false;
            db.force_zero_cooldown_insert = false;
            db.force_next_error = None;
            db.force_insert_event_error = None;
        }
        return Ok((vec![], "ROLLBACK".into()));
    }
    if let Some(err) = db.force_next_error.take() {
        return Err(err);
    }
    if norm.starts_with("CREATE TABLE") || norm.starts_with("CREATE INDEX") {
        return Ok((vec![], "CREATE TABLE".into()));
    }
    exec_event_log(&mut db, &norm, params)
}

fn exec_event_log(db: &mut MockDb, norm: &str, params: &[Option<String>]) -> SqlRes {
    if norm.starts_with("INSERT INTO brokkr_runs") {
        let run_id = pstr(params, 0).to_string();
        if db.runs.contains_key(&run_id) {
            return Err(("23505".into(), "duplicate run_id".into()));
        }
        db.runs.insert(
            run_id,
            (
                pstr(params, 1).into(),
                pstr(params, 2).into(),
                pstr(params, 3).into(),
            ),
        );
        return Ok((vec![], "INSERT 0 1".into()));
    }
    if norm.starts_with("SELECT manifest::text FROM brokkr_runs") {
        let rows = db
            .runs
            .get(pstr(params, 0))
            .map(|(m, _, _)| vec![vec![Some(m.clone())]])
            .unwrap_or_default();
        return Ok((rows, "SELECT 1".into()));
    }
    if norm == "SELECT 1 FROM brokkr_runs WHERE run_id = $1" {
        let rows = if db.runs.contains_key(pstr(params, 0)) {
            vec![vec![Some("1".into())]]
        } else {
            Vec::new()
        };
        return Ok((rows, "SELECT 1".into()));
    }
    if norm.starts_with("SELECT seq, event_hash, envelope::text FROM brokkr_events") {
        let run_id = pstr(params, 0);
        let rows = db
            .events
            .iter()
            .filter(|((r, _), _)| r == run_id)
            .map(|((_, seq), (_, hash, _, _, env))| {
                vec![Some(seq.to_string()), Some(hash.clone()), Some(env.clone())]
            })
            .collect();
        return Ok((rows, "SELECT".into()));
    }
    if norm.starts_with("INSERT INTO brokkr_events") {
        if let Some(err) = db.force_insert_event_error.take() {
            return Err(err);
        }
        let key = (pstr(params, 0).to_string(), pi64(params, 1));
        if db.events.contains_key(&key) {
            return Err(("23505".into(), "duplicate event".into()));
        }
        db.events.insert(
            key,
            (
                pstr(params, 2).into(),
                pstr(params, 3).into(),
                pstr(params, 4).into(),
                pstr(params, 5).into(),
                pstr(params, 6).into(),
            ),
        );
        return Ok((vec![], "INSERT 0 1".into()));
    }
    if norm.starts_with("UPDATE brokkr_runs SET created_at") {
        if let Some(entry) = db.runs.get_mut(pstr(params, 0)) {
            entry.2 = pstr(params, 1).into();
            if !pstr(params, 2).is_empty() {
                entry.1 = pstr(params, 2).into();
            }
        }
        return Ok((vec![], "UPDATE 1".into()));
    }
    if norm == "SELECT run_id FROM brokkr_runs ORDER BY run_id ASC" {
        let rows = db.runs.keys().map(|k| vec![Some(k.clone())]).collect();
        return Ok((rows, "SELECT".into()));
    }
    exec_proj(db, norm, params)
}

fn exec_proj(db: &mut MockDb, norm: &str, params: &[Option<String>]) -> SqlRes {
    if norm.starts_with(
        "SELECT feature, bundle_name, created_at, head_seq, head_hash FROM brokkr_projected_runs",
    ) {
        let rows = db
            .proj_runs
            .get(pstr(params, 0))
            .map(|(f, b, c, s, h)| {
                vec![vec![
                    Some(f.clone()),
                    Some(b.clone()),
                    Some(c.clone()),
                    Some(s.to_string()),
                    Some(h.clone()),
                ]]
            })
            .unwrap_or_default();
        return Ok((rows, "SELECT".into()));
    }
    if norm.starts_with("SELECT COUNT(*) FROM brokkr_projected_events") {
        let run_id = pstr(params, 0);
        let limit = pi64(params, 1);
        let cnt = db
            .proj_events
            .keys()
            .filter(|(r, s)| r == run_id && (*s < 1 || *s > limit))
            .count();
        return Ok((vec![vec![Some(cnt.to_string())]], "SELECT 1".into()));
    }
    if norm.starts_with("SELECT event_hash FROM brokkr_projected_events") {
        let key = (pstr(params, 0).to_string(), pi64(params, 1));
        let rows = db
            .proj_events
            .get(&key)
            .map(|row| vec![vec![Some(row.4.clone())]])
            .unwrap_or_default();
        return Ok((rows, "SELECT".into()));
    }
    if norm.starts_with(
        "SELECT seq, event_id, event_type, attempt_id, member, event_hash, recorded_at, payload::text FROM brokkr_projected_events",
    ) {
        let run_id = pstr(params, 0);
        let limit = pi64(params, 1);
        let rows = db
            .proj_events
            .iter()
            .filter(|((r, s), _)| r == run_id && *s <= limit)
            .map(|((_, s), row)| {
                vec![
                    Some(s.to_string()),
                    Some(row.0.clone()),
                    Some(row.1.clone()),
                    row.2.clone(),
                    row.3.clone(),
                    Some(row.4.clone()),
                    Some(row.5.clone()),
                    Some(row.6.clone()),
                ]
            })
            .collect();
        return Ok((rows, "SELECT".into()));
    }
    exec_proj_writes(db, norm, params)
}

fn exec_proj_writes(db: &mut MockDb, norm: &str, params: &[Option<String>]) -> SqlRes {
    if norm.starts_with("INSERT INTO brokkr_projected_events") {
        let key = (pstr(params, 0).to_string(), pi64(params, 1));
        db.proj_events.insert(
            key,
            (
                pstr(params, 2).into(),
                pstr(params, 3).into(),
                params[4].clone(),
                params[5].clone(),
                pstr(params, 6).into(),
                pstr(params, 7).into(),
                pstr(params, 8).into(),
            ),
        );
        return Ok((vec![], "INSERT 0 1".into()));
    }
    if norm.starts_with("INSERT INTO brokkr_projected_runs") {
        db.proj_runs.insert(
            pstr(params, 0).into(),
            (
                pstr(params, 1).into(),
                pstr(params, 2).into(),
                pstr(params, 3).into(),
                pi64(params, 4),
                pstr(params, 5).into(),
            ),
        );
        return Ok((vec![], "INSERT 0 1".into()));
    }
    if norm.starts_with("SELECT run_id, feature, created_at FROM brokkr_projected_runs") {
        let mut list: Vec<_> = db
            .proj_runs
            .iter()
            .map(|(id, (f, _, c, _, _))| (c.clone(), id.clone(), f.clone()))
            .collect();
        list.sort();
        let rows = list
            .into_iter()
            .map(|(c, id, f)| vec![Some(id), Some(f), Some(c)])
            .collect();
        return Ok((rows, "SELECT".into()));
    }
    if norm == "DELETE FROM brokkr_projected_events" {
        db.proj_events.clear();
        return Ok((vec![], "DELETE 0".into()));
    }
    if norm == "DELETE FROM brokkr_projected_runs" {
        db.proj_runs.clear();
        return Ok((vec![], "DELETE 0".into()));
    }
    exec_coord(db, norm, params)
}

fn exec_coord(db: &mut MockDb, norm: &str, params: &[Option<String>]) -> SqlRes {
    if norm == "SELECT 1 FROM brokkr_queue_entries WHERE id = $1::bigint" {
        let rows = if db.queue_entries.contains_key(&pi64(params, 0)) {
            vec![vec![Some("1".into())]]
        } else {
            Vec::new()
        };
        return Ok((rows, "SELECT".into()));
    }
    if norm.starts_with("INSERT INTO brokkr_queue_entries") {
        if db.force_empty_returning {
            db.force_empty_returning = false;
            return Ok((vec![], "INSERT 0 0".into()));
        }
        db.queue_next_id += 1;
        let id = db.queue_next_id;
        db.queue_entries.insert(
            id,
            (
                pstr(params, 0).into(),
                pi64(params, 1),
                "pending".into(),
                None,
                None,
                None,
                pstr(params, 2).into(),
            ),
        );
        return Ok((vec![vec![Some(id.to_string())]], "INSERT 0 1".into()));
    }
    if norm.starts_with("INSERT INTO brokkr_queue_waits") {
        db.queue_waits
            .insert((pi64(params, 0), pi64(params, 1)), pstr(params, 2).into());
        return Ok((vec![], "INSERT 0 1".into()));
    }
    if norm.starts_with(
        "SELECT id, status, priority, payload, added_at, claimed_by, run_id FROM brokkr_queue_entries",
    ) {
        let mut items: Vec<_> = db
            .queue_entries
            .iter()
            .filter(|(_, e)| e.2 != "dropped")
            .map(|(&id, e)| {
                let rank = if matches!(e.2.as_str(), "pending" | "held" | "launching") {
                    0
                } else {
                    1
                };
                (rank, std::cmp::Reverse(e.1), id, e.clone())
            })
            .collect();
        items.sort_by_key(|(r, p, id, _)| (*r, *p, *id));
        let rows = items
            .into_iter()
            .map(|(_, _, id, e)| {
                vec![
                    Some(id.to_string()),
                    Some(e.2),
                    Some(e.1.to_string()),
                    Some(e.0),
                    Some(e.6),
                    e.3,
                    e.5,
                ]
            })
            .collect();
        return Ok((rows, "SELECT".into()));
    }
    exec_coord_claims(db, norm, params)
}

fn exec_coord_claims(db: &mut MockDb, norm: &str, params: &[Option<String>]) -> SqlRes {
    if norm.starts_with("SELECT entry_id, awaits, condition FROM brokkr_queue_waits") {
        let rows = db
            .queue_waits
            .iter()
            .map(|((e, a), c)| vec![Some(e.to_string()), Some(a.to_string()), Some(c.clone())])
            .collect();
        return Ok((rows, "SELECT".into()));
    }
    if norm.starts_with("SELECT awaits, condition FROM brokkr_queue_waits") {
        let target = pi64(params, 0);
        let rows = db
            .queue_waits
            .iter()
            .filter(|((e, _), _)| *e == target)
            .map(|((_, a), c)| vec![Some(a.to_string()), Some(c.clone())])
            .collect();
        return Ok((rows, "SELECT".into()));
    }
    if norm.contains("FOR UPDATE SKIP LOCKED") {
        let mut pending: Vec<_> = db
            .queue_entries
            .iter()
            .filter(|(id, e)| e.2 == "pending" && !db.locked_queue_ids.contains(id))
            .map(|(&id, e)| (std::cmp::Reverse(e.1), id, e.clone()))
            .collect();
        pending.sort_by_key(|(p, id, _)| (*p, *id));
        let rows = pending
            .into_iter()
            .next()
            .map(|(_, id, e)| {
                vec![vec![
                    Some(id.to_string()),
                    Some(e.0),
                    Some(e.1.to_string()),
                    Some(e.6),
                ]]
            })
            .unwrap_or_default();
        return Ok((rows, "SELECT".into()));
    }
    if norm.starts_with("UPDATE brokkr_queue_entries SET status = 'claimed'") {
        if let Some(e) = db.queue_entries.get_mut(&pi64(params, 0)) {
            e.2 = "claimed".into();
            e.3 = Some(pstr(params, 1).into());
            e.4 = Some(pstr(params, 2).into());
        }
        return Ok((vec![], "UPDATE 1".into()));
    }
    exec_cooldowns(db, norm, params)
}

fn exec_cooldowns(db: &mut MockDb, norm: &str, params: &[Option<String>]) -> SqlRes {
    if norm.starts_with("SELECT seq, site, provider, route, until_at, reason") {
        let now = pi64(params, 0);
        let rows = db
            .cooldowns
            .iter()
            .filter(|(_, c)| c.5 > now)
            .map(|(&seq, c)| {
                vec![
                    Some(seq.to_string()),
                    Some(c.2.clone()),
                    Some(c.3.clone()),
                    Some(c.4.clone()),
                    Some(c.5.to_string()),
                    Some(c.6.clone()),
                    c.7.map(|v| v.to_string()),
                    Some(c.8.to_string()),
                    Some(c.9.to_string()),
                ]
            })
            .collect();
        return Ok((rows, "SELECT".into()));
    }
    if norm.starts_with("INSERT INTO brokkr_provider_cooldowns") {
        if db.force_zero_cooldown_insert {
            db.force_zero_cooldown_insert = false;
            return Ok((vec![], "INSERT 0 0".into()));
        }
        let key = (
            pstr(params, 0),
            pi64(params, 1),
            pstr(params, 2),
            pi64(params, 8),
        );
        let dup = db
            .cooldowns
            .values()
            .any(|c| c.0 == key.0 && c.1 == key.1 && c.2 == key.2 && c.8 == key.3);
        if dup {
            return Ok((vec![], "INSERT 0 0".into()));
        }
        db.cooldown_next_seq += 1;
        let seq = db.cooldown_next_seq;
        db.cooldowns.insert(
            seq,
            (
                pstr(params, 0).into(),
                pi64(params, 1),
                pstr(params, 2).into(),
                pstr(params, 3).into(),
                pstr(params, 4).into(),
                pi64(params, 5),
                pstr(params, 6).into(),
                params[7].as_deref().map(|s| s.parse().unwrap()),
                pi64(params, 8),
                pi64(params, 9),
            ),
        );
        return Ok((vec![], "INSERT 0 1".into()));
    }
    if norm.starts_with(
        "SELECT provider, route, until_at, reason, retry_after_seconds, recorded_at FROM brokkr_provider_cooldowns",
    ) {
        let rows = db
            .cooldowns
            .values()
            .find(|c| {
                c.0 == pstr(params, 0)
                    && c.1 == pi64(params, 1)
                    && c.2 == pstr(params, 2)
                    && c.8 == pi64(params, 3)
            })
            .map(|c| {
                vec![vec![
                    Some(c.3.clone()),
                    Some(c.4.clone()),
                    Some(c.5.to_string()),
                    Some(c.6.clone()),
                    c.7.map(|v| v.to_string()),
                    Some(c.9.to_string()),
                ]]
            })
            .unwrap_or_default();
        return Ok((rows, "SELECT".into()));
    }
    Err(("42601".into(), format!("unhandled mock sql: {norm}")))
}
