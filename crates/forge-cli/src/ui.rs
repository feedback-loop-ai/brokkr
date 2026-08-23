//! `forge ui` — the embedded read-only surface (decision 0003): a static
//! page compiled into the binary, served on loopback, reading
//! projections only. No commands, no writes, no Node, no external
//! assets; removing this module changes nothing about execution
//! semantics (first-release acceptance criteria).
//!
//! Routes: `/` (page) · `/api/runs` · `/api/run/<id>` · `/sse/<id>`
//! (server-sent head changes, poll-backed).

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;

use forge_core::fold::{fold, Status};
use forge_store::Store;
use serde_json::{json, Value};

const PAGE: &str = include_str!("ui.html");

pub struct Response {
    pub status: &'static str,
    pub content_type: &'static str,
    pub body: String,
}

fn ok(content_type: &'static str, body: String) -> Response {
    Response {
        status: "200 OK",
        content_type,
        body,
    }
}

fn not_found(what: &str) -> Response {
    Response {
        status: "404 Not Found",
        content_type: "application/json",
        body: json!({"error": format!("{what} not found")}).to_string(),
    }
}

fn status_str(status: &Status) -> &'static str {
    match status {
        Status::Running => "running",
        Status::AwaitingOperator => "awaiting_operator",
        Status::Completed => "completed",
        Status::Stopped => "stopped",
    }
}

/// Pure request handling: path in, response out. The TCP loop below is a
/// thin shell around this, which is what the tests exercise.
pub fn handle(db: &PathBuf, path: &str) -> Response {
    if path == "/" {
        return ok("text/html; charset=utf-8", PAGE.to_string());
    }
    let store = match Store::open(db) {
        Ok(store) => store,
        Err(e) => {
            return Response {
                status: "500 Internal Server Error",
                content_type: "application/json",
                body: json!({"error": e.to_string()}).to_string(),
            }
        }
    };
    if path == "/api/runs" {
        let mut runs = Vec::new();
        if let Ok(list) = store.list_runs() {
            for (run_id, feature, created_at) in list {
                let state = store.load(&run_id).ok().and_then(|e| fold(&e).ok());
                runs.push(json!({
                    "run_id": run_id,
                    "feature": feature,
                    "created_at": created_at,
                    "status": state.as_ref().map(|s| status_str(&s.status)),
                    "phase": state.as_ref().and_then(|s| s.phase.clone()),
                    "seq": state.as_ref().map(|s| s.seq),
                }));
            }
        }
        return ok("application/json", Value::Array(runs).to_string());
    }
    if let Some(run_id) = path.strip_prefix("/api/run/") {
        let events = match store.load(run_id) {
            Ok(events) => events,
            Err(_) => return not_found(run_id),
        };
        let state = fold(&events).ok();
        let body = json!({
            "summary": state.map(|s| json!({
                "run_id": s.run_id,
                "status": status_str(&s.status),
                "phase": s.phase,
                "seq": s.seq,
                "park_reason": s.park_reason,
                "consecutive_failures": s.consecutive_failures,
                "last_decision": s.last_decision,
                "feature": s.feature,
            })),
            "events": events,
        });
        return ok("application/json", body.to_string());
    }
    not_found(path)
}

fn head_seq(db: &PathBuf, run_id: &str) -> u64 {
    Store::open(db)
        .and_then(|s| s.head_hash(run_id))
        .map(|(seq, _)| seq)
        .unwrap_or(0)
}

fn serve_client(db: PathBuf, stream: TcpStream) {
    let mut reader = BufReader::new(match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    });
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() {
        return;
    }
    let path = line.split_whitespace().nth(1).unwrap_or("/").to_string();
    let mut stream = stream;

    if let Some(run_id) = path.strip_prefix("/sse/") {
        let run_id = run_id.to_string();
        let header = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\
                      Cache-Control: no-cache\r\nConnection: keep-alive\r\n\r\n";
        if stream.write_all(header.as_bytes()).is_err() {
            return;
        }
        let mut last = u64::MAX; // always push one initial event
        loop {
            let seq = head_seq(&db, &run_id);
            if seq != last {
                last = seq;
                let message = format!("data: {}\n\n", json!({"seq": seq}));
                if stream.write_all(message.as_bytes()).is_err() {
                    return; // client went away
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(1000));
        }
    }

    let response = handle(&db, &path);
    let payload = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\
         Connection: close\r\n\r\n{}",
        response.status,
        response.content_type,
        response.body.len(),
        response.body
    );
    let _ = stream.write_all(payload.as_bytes());
}

/// Bind loopback and serve until killed. Returns the bound port (0 in
/// `port` picks an ephemeral one).
pub fn serve(db: PathBuf, port: u16, open_browser: bool) -> std::io::Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    let bound = listener.local_addr()?.port();
    eprintln!("forge ui: http://127.0.0.1:{bound}/ (read-only; Ctrl-C to stop)");
    if open_browser {
        let _ = std::process::Command::new("xdg-open")
            .arg(format!("http://127.0.0.1:{bound}/"))
            .spawn();
    }
    for stream in listener.incoming().flatten() {
        let db = db.clone();
        std::thread::spawn(move || serve_client(db, stream));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_core::envelope::EventType;
    use serde_json::json;

    fn fixture() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("forge.db");
        let mut store = Store::open(&db).unwrap();
        store.create_run("r1", "feat", "self", &json!({"files": {}})).unwrap();
        store
            .append_next("r1", EventType::RunStarted,
                json!({"feature": "feat", "manifest": {}}), None, None)
            .unwrap();
        store
            .append_next("r1", EventType::PhaseEntered,
                json!({"phase": "intake"}), None, None)
            .unwrap();
        (dir, db)
    }

    #[test]
    fn page_runs_and_run_detail_serve_read_only() {
        let (_dir, db) = fixture();
        let page = handle(&db, "/");
        assert_eq!(page.status, "200 OK");
        assert!(page.body.contains("<title>forge</title>"));

        let runs = handle(&db, "/api/runs");
        let parsed: Value = serde_json::from_str(&runs.body).unwrap();
        assert_eq!(parsed[0]["run_id"], "r1");
        assert_eq!(parsed[0]["phase"], "intake");

        let detail = handle(&db, "/api/run/r1");
        let parsed: Value = serde_json::from_str(&detail.body).unwrap();
        assert_eq!(parsed["summary"]["status"], "running");
        assert_eq!(parsed["events"].as_array().unwrap().len(), 2);

        assert_eq!(handle(&db, "/api/run/nope").status, "404 Not Found");
        assert_eq!(handle(&db, "/definitely-not").status, "404 Not Found");
    }
}
