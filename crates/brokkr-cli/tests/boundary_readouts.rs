//! Decision 0046 ruling 3: every readout that names a seat's model names
//! the boundary its hands stood behind, and a run whose gate stood under
//! `harness` or `open` is rendered *unboxed* wherever the run is
//! summarised.
//!
//! Two halves. The pin: a `roster.rs`-style read of every readout source
//! that fails, naming the source, where `served.model` is read outside
//! the one pair helper's two faces, where a seat-costs record's `model`
//! key is emitted without `boundary` beside it, or where `ui.html` reads
//! `.model` off a carrier outside its page-side pair helper (design
//! DD12). The readouts: a journal planted with the entries and stamps
//! the engine writes, read back through the binary — `inspect`, `seats`
//! in both faces, `costs`, `compare`, and `export` with `verify-run`.

use std::path::{Path, PathBuf};
use std::process::Command;

use brokkr_core::EventType;
use brokkr_store::Store;
use serde_json::{json, Value};

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn source(relative: &str) -> String {
    std::fs::read_to_string(workspace().join(relative)).unwrap()
}

/// The lines of one function body: from the line carrying `signature`
/// to the first line after it that is exactly `}` at column zero. What
/// the pin exempts, and nothing else.
fn body_lines(text: &str, signature: &str) -> std::ops::RangeInclusive<usize> {
    let start = text
        .lines()
        .position(|line| line.contains(signature))
        .unwrap_or_else(|| panic!("{signature} is defined"));
    let end = text
        .lines()
        .enumerate()
        .skip(start)
        .find(|(_, line)| *line == "}")
        .map(|(index, _)| index)
        .unwrap();
    start..=end
}

fn is_comment(line: &str) -> bool {
    line.trim_start().starts_with("//")
}

/// The pair helper is the only place a Rust readout reads a served
/// model cell: `served.model`, or a carrier's `.model.text`, anywhere
/// else is a surface that could show the model without the boundary.
#[test]
fn no_rust_readout_reads_the_model_cell_outside_the_pair_helper() {
    let render = source("crates/brokkr-cli/src/render.rs");
    let text_face = body_lines(&render, "pub(crate) fn served_text(");
    let json_face = body_lines(&render, "pub(crate) fn served_json(");
    for (name, text) in [
        ("crates/brokkr-cli/src/render.rs", render.as_str()),
        (
            "crates/brokkr-cli/src/tui.rs",
            &source("crates/brokkr-cli/src/tui.rs"),
        ),
        (
            "crates/brokkr-cli/src/compare.rs",
            &source("crates/brokkr-cli/src/compare.rs"),
        ),
    ] {
        for (index, line) in text.lines().enumerate() {
            if is_comment(line) {
                continue;
            }
            let exempt = name.ends_with("render.rs")
                && (text_face.contains(&index) || json_face.contains(&index));
            if exempt {
                continue;
            }
            for pattern in [
                "served.model",
                ".model.text",
                ".model.absent",
                ".model.note",
            ] {
                assert!(
                    !line.contains(pattern),
                    "{name}:{} reads `{pattern}` outside the pair helper: {line}",
                    index + 1
                );
            }
        }
    }
    // The faces themselves are where the reads live.
    let faces: String = render
        .lines()
        .enumerate()
        .filter(|(index, _)| text_face.contains(index) || json_face.contains(index))
        .map(|(_, line)| line)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(faces.contains("served.model"), "{faces}");
    assert!(faces.contains("served.boundary"), "{faces}");
}

/// A seat-costs record that names `model` names `boundary` beside it.
#[test]
fn the_seat_costs_record_emits_the_boundary_beside_the_model() {
    let compare = source("crates/brokkr-cli/src/compare.rs");
    let model_at = compare
        .find("(\"model\".to_string(), Value::from(model))")
        .expect("the seat-costs record names its model");
    let after = &compare[model_at..];
    let boundary_at = after
        .find("(\"boundary\".to_string(), Value::from(boundary))")
        .expect("the seat-costs record names its boundary");
    assert!(
        boundary_at < 200,
        "crates/brokkr-cli/src/compare.rs emits the `model` key of a seat-costs record \
         without `boundary` beside it"
    );
}

/// The page carries one pair helper — `served(carrier)` — and it is the
/// only place `ui.html` names `.model`. Any other read is named by line.
#[test]
fn the_page_reads_the_model_only_through_its_pair_helper() {
    let page = source("crates/brokkr-cli/src/ui.html");
    let helper = body_lines(&page, "function served(carrier)");
    let mut inside = 0usize;
    for (index, line) in page.lines().enumerate() {
        if is_comment(line) {
            continue;
        }
        if helper.contains(&index) {
            inside += usize::from(line.contains(".model"));
            continue;
        }
        assert!(
            !line.contains(".model"),
            "crates/brokkr-cli/src/ui.html:{} reads `.model` off a carrier outside the \
             page-side pair helper `served(carrier)`: {line}",
            index + 1
        );
    }
    assert_eq!(inside, 1, "the helper reads the model exactly once");
}

// ------------------------------------------------------------ the readouts

fn brokkr_bin() -> &'static str {
    env!("CARGO_BIN_EXE_brokkr")
}

fn brokkr(dir: &Path, args: &[&str]) -> (String, String, bool) {
    let out = Command::new(brokkr_bin())
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.success(),
    )
}

/// One `verify` seat under `word`, planted as the engine records it: the
/// manifest declares hands, the attempt's `effect/started` carries the
/// entry, and every record that names a model carries the word beside
/// it (design DD19). `None` plants a journal written before the boundary
/// was named: hands declared, no entry, no stamp.
fn plant(db: &Path, run_id: &str, word: Option<&str>) {
    let stamped = |mut record: Value| {
        if let Some(word) = word {
            record["boundary"] = json!(word);
        }
        record
    };
    let mut store = Store::open(db).unwrap();
    store
        .create_run(run_id, "box it", "test", &json!({"engine": "0.9.0"}))
        .unwrap();
    let mut append = |kind, payload| {
        store
            .append_next(run_id, kind, payload, None, None)
            .unwrap();
    };
    append(
        EventType::RunStarted,
        json!({"feature": "box it", "manifest": {"engine": "0.9.0",
               "hands": {"verify": {"binds": []}}}}),
    );
    append(EventType::PhaseEntered, json!({"phase": "verify"}));
    append(
        EventType::EffectRequested,
        json!({"effect_id": "e1", "seat": "verify", "phase": "verify"}),
    );
    let mut started = json!({"effect_id": "e1", "attempt_id": "a1", "driver": "d"});
    if let Some(word) = word {
        started["boundary"] = json!([{"member": null, "boundary": word, "gate": true}]);
    }
    append(EventType::EffectStarted, started);
    append(
        EventType::EffectCheckpointed,
        json!({"effect_id": "e1", "attempt_id": "a1", "checkpoint": stamped(json!({
            "step": "seat-turn", "turn": 1, "model": "claude-fable-5-1"}))}),
    );
    append(
        EventType::EffectSucceeded,
        json!({"effect_id": "e1", "attempt_id": "a1", "result": stamped(json!({
            "result": "pass", "model": "claude-fable-5-1"}))}),
    );
}

/// `brokkr seats` renders the seats block `inspect` renders, and its
/// `--json` is `inspect --json`'s bytes; `inspect`'s header and trail
/// carry the boundary from the one derivation.
#[test]
fn brokkr_seats_prints_inspects_seats_block_and_inspects_json_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    plant(&db, "harnessed", Some("harness"));
    let db = db.to_str().unwrap();

    let (inspect, _, ok) = brokkr(dir.path(), &["inspect", "--run", "harnessed", "--db", db]);
    assert!(ok, "{inspect}");
    assert!(
        inspect.contains("     boundary harness · unboxed\n"),
        "{inspect}"
    );
    assert!(inspect.contains("claude-fable-5-1 harness "), "{inspect}");
    assert!(
        inspect.contains("· model claude-fable-5-1 · boundary harness"),
        "{inspect}"
    );

    let (seats, _, ok) = brokkr(dir.path(), &["seats", "--run", "harnessed", "--db", db]);
    assert!(ok, "{seats}");
    assert!(seats.starts_with("seats\n"), "{seats}");
    assert!(
        seats.contains(" model ") && seats.contains(" boundary "),
        "{seats}"
    );
    assert!(
        inspect.contains(&format!("\n{seats}\n")),
        "{inspect}\n---\n{seats}"
    );
    assert!(!seats.contains("run  "), "the block alone: {seats}");

    let (seats_json, _, ok) = brokkr(
        dir.path(),
        &["seats", "--run", "harnessed", "--db", db, "--json"],
    );
    assert!(ok, "{seats_json}");
    let (inspect_json, _, _) = brokkr(
        dir.path(),
        &["inspect", "--run", "harnessed", "--db", db, "--json"],
    );
    assert_eq!(seats_json, inspect_json, "byte-identical");
    let view: Value = serde_json::from_str(&seats_json).unwrap();
    assert_eq!(view["view_version"], brokkr_view::VIEW_VERSION);
    assert_eq!(view["participants"][0]["boundary"]["text"], "harness");
    assert_eq!(view["boundary"]["text"], "harness · unboxed");
    assert_eq!(view["boundary"]["unboxed"], json!(true));
    assert_eq!(view["boundary"]["word"]["text"], "harness");
}

/// `costs` and `compare` carry `boundary` beside `model` in every
/// per-seat record, `costs` printing the plain word; `compare` reports a
/// boundary difference as a divergence in both the seat records and the
/// resolution map; and a pre-0046 seat reads `not recorded`.
#[test]
fn costs_and_compare_name_the_boundary_and_diverge_on_it() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    plant(&db, "harnessed", Some("harness"));
    plant(&db, "boxed", Some("namespace"));
    plant(&db, "older", None);
    let db = db.to_str().unwrap();

    let (costs, _, ok) = brokkr(dir.path(), &["costs", "--run", "harnessed", "--db", db]);
    assert!(ok, "{costs}");
    let costs: Value = serde_json::from_str(&costs).unwrap();
    assert_eq!(costs["seats"]["verify"]["model"], "claude-fable-5-1");
    assert_eq!(costs["seats"]["verify"]["boundary"], "harness");
    let (older, _, _) = brokkr(dir.path(), &["costs", "--run", "older", "--db", db]);
    let older: Value = serde_json::from_str(&older).unwrap();
    assert_eq!(older["seats"]["verify"]["model"], "claude-fable-5-1");
    assert_eq!(older["seats"]["verify"]["boundary"], "not recorded");

    let (report, _, ok) = brokkr(dir.path(), &["compare", "harnessed", "boxed", "--db", db]);
    assert!(ok, "{report}");
    let report: Value = serde_json::from_str(&report).unwrap();
    assert_eq!(
        report["runs"]["harnessed"]["seats"]["verify"]["boundary"],
        "harness"
    );
    assert_eq!(
        report["runs"]["boxed"]["seats"]["verify"]["boundary"],
        "namespace"
    );
    assert_eq!(
        report["runs"]["harnessed"]["resolution"]["verify"]["boundary"],
        "harness"
    );
    let divergence = &report["comparison"]["resolution_divergence"];
    assert_eq!(divergence["verify"]["a"]["boundary"], "harness");
    assert_eq!(divergence["verify"]["b"]["boundary"], "namespace");
    assert_eq!(divergence["verify"]["a"]["model"], "claude-fable-5-1");
    assert!(
        !report.to_string().contains("unboxed"),
        "data, not adjective"
    );

    let (same, _, _) = brokkr(dir.path(), &["compare", "boxed", "boxed", "--db", db]);
    let same: Value = serde_json::from_str(&same).unwrap();
    assert_eq!(same["comparison"]["resolution_divergence"], json!({}));
}

/// `export` is the record itself: the exported events and seat records
/// carry the plain word, `verify-run` accepts the file, and no adjective
/// appears in it.
#[test]
fn export_carries_the_word_as_data_and_verify_run_accepts_it() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    plant(&db, "harnessed", Some("harness"));
    let out = dir.path().join("export");
    let (_, stderr, ok) = brokkr(
        dir.path(),
        &[
            "export",
            "--run",
            "harnessed",
            "--out",
            out.to_str().unwrap(),
            "--db",
            db.to_str().unwrap(),
        ],
    );
    assert!(ok, "{stderr}");
    let file = out.join("harnessed.ndjson");
    let ndjson = std::fs::read_to_string(&file).unwrap();
    let events: Vec<Value> = ndjson
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let started = events
        .iter()
        .find(|event| event["type"] == "effect/started")
        .unwrap();
    assert_eq!(started["payload"]["boundary"][0]["boundary"], "harness");
    assert_eq!(started["payload"]["boundary"][0]["gate"], json!(true));
    let checkpoint = events
        .iter()
        .find(|event| event["type"] == "effect/checkpointed")
        .unwrap();
    assert_eq!(checkpoint["payload"]["checkpoint"]["boundary"], "harness");
    let succeeded = events
        .iter()
        .find(|event| event["type"] == "effect/succeeded")
        .unwrap();
    assert_eq!(succeeded["payload"]["result"]["boundary"], "harness");
    assert!(!ndjson.contains("unboxed"), "{ndjson}");

    let (_, stderr, ok) = brokkr(dir.path(), &["verify-run", file.to_str().unwrap()]);
    assert!(ok, "{stderr}");
}
