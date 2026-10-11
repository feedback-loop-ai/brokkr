//! One derivation, two surfaces (decision 0013; decision 0071 ruling 7):
//! what `brokkr-view` exports, the surfaces only paint (#351). No file
//! under `crates/brokkr-cli/src`, its Rust and `ui.html` alike, restates
//! a derivation the view took over. That the engine admits nothing by
//! the view is `layering`'s to hold: it reads the crate graph. The shared
//! transcript reader is held the same way: `local_transcript/` resolves no
//! `HOME`, and the browser's module defines none of it.

use std::path::Path;

use crate::workspace_root::workspace;

/// Each derivation the view took over, by a line a restatement of it
/// cannot avoid, and the one home a surface calls instead.
const RESTATED: [(&str, &str); 10] = [
    ("fn seat_costs", "brokkr_view::seat_costs"),
    ("fn first_divergence", "brokkr_view::first_divergence"),
    (
        "fn resolution_divergence",
        "brokkr_view::resolution_divergence",
    ),
    ("fn status_str", "Status::as_str"),
    ("Status::Stopped => \"stopped\"", "Status::as_str"),
    ("format!(\"{:?}\", state.cursor)", "brokkr_view::summary"),
    (
        "=> LegacyProvenance::LaneTally",
        "Participant::legacy_provenance",
    ),
    ("status == \"working\"", "the served Participant.working"),
    ("status === 'working'", "the served Participant.working"),
    ("legacy_id: part.session_id", "brokkr_view::Subject::of"),
];

/// The scope rule the view took over (#351, ruled 2026-10-10: the TUI
/// leads), by its Rust definitions and by the browser's old predicates:
/// a participant's phase compared, a row's phases searched, a seat
/// matched by its key alone. The surfaces call `brokkr_view::lens_for`
/// and the `keeps_*` predicates, and the page reads `RunView.scopes`.
const SCOPING: [(&str, &str); 9] = [
    ("enum Scope", "brokkr_view::Scope"),
    ("struct Lens", "brokkr_view::Lens"),
    ("fn lens_for", "brokkr_view::lens_for"),
    ("fn keeps_phase", "brokkr_view::keeps_phase"),
    ("fn keeps_participant", "brokkr_view::keeps_participant"),
    ("fn keeps_row", "brokkr_view::keeps_row"),
    (".phase === phaseFilter", "the served RunView.scopes"),
    ("phases.includes(", "the served RunView.scopes"),
    ("|| part.key ===", "the served RunView.scopes"),
];

/// The ambient home the shared reader never resolves (#351): the shell
/// resolves the projects home once and passes it to `read_local`.
const AMBIENT_HOME: [(&str, &str); 3] = [
    ("var_os(\"HOME\")", "the shell's local_projects_home"),
    ("var(\"HOME\")", "the shell's local_projects_home"),
    ("home_dir(", "the shell's local_projects_home"),
];

/// The reader's own definitions, which only `local_transcript/` holds:
/// `ui.rs` keeps the routes and calls them (#351).
const READER: [(&str, &str); 3] = [
    ("fn discover", "local_transcript::discover"),
    ("fn read_local", "local_transcript::read_local"),
    ("mod safe_fs", "local_transcript's safe_fs"),
];

/// Each of `table`'s lines `text` holds, as `shown: why`.
fn restated_in(shown: &str, text: &str, table: &[(&str, &str)]) -> Vec<String> {
    table
        .iter()
        .filter(|(line, _)| text.contains(line))
        .map(|(line, home)| format!("{shown}: restates `{line}`; call {home}"))
        .collect()
}

/// Every restatement of `table`'s lines in the files under `root`, as
/// `path: why`, sorted. A file the scan cannot read as text is a finding
/// too: what it cannot read, it cannot clear.
fn restatements(root: &Path, table: &[(&str, &str)]) -> Vec<String> {
    let mut out = Vec::new();
    let mut dirs = vec![root.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        let entries = std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
        for entry in entries {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                dirs.push(path);
                continue;
            }
            let shown = path.strip_prefix(root).expect("under the root").display();
            match std::fs::read_to_string(&path) {
                Ok(text) => out.extend(restated_in(&shown.to_string(), &text, table)),
                Err(error) => out.push(format!("{shown}: not readable as text ({error})")),
            }
        }
    }
    out.sort();
    out
}

#[test]
fn the_surfaces_restate_no_derivation_the_view_exports() {
    let src = workspace().join("crates/brokkr-cli/src");
    assert_eq!(restatements(&src, &RESTATED), Vec::<String>::new());
}

/// #351 ask 2: the terminal, the TUI and the browser scope a run by the
/// view's one rule, and none of them defines or restates it.
#[test]
fn the_surfaces_restate_no_scope_rule() {
    let src = workspace().join("crates/brokkr-cli/src");
    assert_eq!(restatements(&src, &SCOPING), Vec::<String>::new());
}

/// The scope scan bites: the CLI's old definitions and the browser's old
/// predicates, planted, are each named with the view's home. The plants
/// and the diagnostics are written out, not read from `SCOPING`.
#[test]
fn the_scope_scan_names_every_definition_and_every_browser_predicate() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("render.rs"),
        "pub(crate) enum Scope {}\npub(crate) struct Lens {}\nfn lens_for() {}\n\
         fn keeps_phase() {}\nfn keeps_participant() {}\nfn keeps_row() {}\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("ui.html"),
        "parts.filter((part) => !phaseFilter || part.phase === phaseFilter)\n\
         parts.filter((part) => !activeParticipant || part.key === activeParticipant);\n\
         if (phaseFilter && !e.phases.includes(phaseFilter)) continue;\n",
    )
    .unwrap();
    assert_eq!(
        restatements(dir.path(), &SCOPING),
        [
            "render.rs: restates `enum Scope`; call brokkr_view::Scope",
            "render.rs: restates `fn keeps_participant`; call brokkr_view::keeps_participant",
            "render.rs: restates `fn keeps_phase`; call brokkr_view::keeps_phase",
            "render.rs: restates `fn keeps_row`; call brokkr_view::keeps_row",
            "render.rs: restates `fn lens_for`; call brokkr_view::lens_for",
            "render.rs: restates `struct Lens`; call brokkr_view::Lens",
            "ui.html: restates `.phase === phaseFilter`; call the served RunView.scopes",
            "ui.html: restates `phases.includes(`; call the served RunView.scopes",
            "ui.html: restates `|| part.key ===`; call the served RunView.scopes",
        ]
    );
}

/// #351 ask 6: the reader takes the projects home as an argument and
/// resolves none itself, and `ui.rs` keeps only the routes and serving.
#[test]
fn the_reader_resolves_no_home_and_ui_defines_none_of_it() {
    let src = workspace().join("crates/brokkr-cli/src");
    assert_eq!(
        restatements(&src.join("local_transcript"), &AMBIENT_HOME),
        Vec::<String>::new()
    );
    let ui = std::fs::read_to_string(src.join("ui.rs")).expect("ui.rs reads as text");
    assert_eq!(restated_in("ui.rs", &ui, &READER), Vec::<String>::new());
}

/// Both of the reader's scans bite: an ambient home read planted anywhere
/// under the reader, and each reader definition planted in the browser's
/// module, is named, and an unreadable file is refused by name. The plants
/// and the diagnostics are written out, not read from the tables.
#[test]
fn the_reader_scans_name_every_home_read_and_every_reader_definition() {
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("discover");
    std::fs::create_dir(&nested).unwrap();
    std::fs::write(
        nested.join("walk.rs"),
        "let home = std::env::var_os(\"HOME\")?;\nlet other = env::var(\"HOME\");\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("mod.rs"), "let home = dirs::home_dir();").unwrap();
    std::fs::write(dir.path().join("safe_fs.rs"), b"\xff\xfe").unwrap();
    assert_eq!(
        restatements(dir.path(), &AMBIENT_HOME),
        [
            r#"discover/walk.rs: restates `var("HOME")`; call the shell's local_projects_home"#,
            r#"discover/walk.rs: restates `var_os("HOME")`; call the shell's local_projects_home"#,
            "mod.rs: restates `home_dir(`; call the shell's local_projects_home",
            "safe_fs.rs: not readable as text (stream did not contain valid UTF-8)",
        ]
    );
    let planted = "mod safe_fs;\nfn discover_claude(root: &Dir) {}\npub fn read_local() {}\n";
    assert_eq!(
        restated_in("ui.rs", planted, &READER),
        [
            "ui.rs: restates `fn discover`; call local_transcript::discover",
            "ui.rs: restates `fn read_local`; call local_transcript::read_local",
            "ui.rs: restates `mod safe_fs`; call local_transcript's safe_fs",
        ]
    );
}

/// The scan bites: each restatement it exists to refuse, planted, is
/// named with its home, and a file it cannot read is refused by name.
/// The planted lines and the diagnostics are written out, not read from
/// `RESTATED`, so an entry dropped from the table fails here.
#[test]
fn the_scan_names_every_restatement_and_every_file_it_cannot_read() {
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("tui");
    std::fs::create_dir(&nested).unwrap();
    let planted = r#"fn seat_costs(events: &[Event]) {}
fn first_divergence(a: &[Event], b: &[Event]) {}
fn resolution_divergence(a: &Value, b: &Value) {}
fn status_str(status: Status) {}
    Status::Stopped => "stopped",
let cursor = format!("{:?}", state.cursor);
    "lanetally" => LegacyProvenance::LaneTally,
let working = part.status == "working";
const working = part.status === 'working';
Subject { legacy_id: part.session_id }
"#;
    std::fs::write(nested.join("state.rs"), planted).unwrap();
    std::fs::write(dir.path().join("ui.html"), b"\xff\xfe").unwrap();
    std::fs::write(
        dir.path().join("lib.rs"),
        "brokkr_view::seat_costs(&events)",
    )
    .unwrap();
    // A call through the view is no restatement.
    assert_eq!(
        restatements(dir.path(), &RESTATED),
        [
            "tui/state.rs: restates `=> LegacyProvenance::LaneTally`; call Participant::legacy_provenance",
            r#"tui/state.rs: restates `Status::Stopped => "stopped"`; call Status::as_str"#,
            "tui/state.rs: restates `fn first_divergence`; call brokkr_view::first_divergence",
            "tui/state.rs: restates `fn resolution_divergence`; call brokkr_view::resolution_divergence",
            "tui/state.rs: restates `fn seat_costs`; call brokkr_view::seat_costs",
            "tui/state.rs: restates `fn status_str`; call Status::as_str",
            r#"tui/state.rs: restates `format!("{:?}", state.cursor)`; call brokkr_view::summary"#,
            "tui/state.rs: restates `legacy_id: part.session_id`; call brokkr_view::Subject::of",
            r#"tui/state.rs: restates `status == "working"`; call the served Participant.working"#,
            "tui/state.rs: restates `status === 'working'`; call the served Participant.working",
            "ui.html: not readable as text (stream did not contain valid UTF-8)",
        ]
    );
}
