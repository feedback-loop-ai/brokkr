//! Decision 0063, held by a test rather than by review (#356): the hosts
//! are Linux and macOS, so no tracked file outside the records named in
//! [`RECORDS`] spells a Windows conditional. The whole tracked tree is
//! read, not a pull request's diff, so the check is the same in a pull
//! request, in the merge queue and on `main`.
//!
//! Every line is read with its whitespace dropped. A Rust source is read a
//! second way, by unit, with the lexer `suppressions.rs` shares
//! (`support/rust_source.rs`): each attribute (`#[...]` or `#![...]`) and
//! each `cfg!` invocation is read whole, from its `#` or `cfg` to the
//! bracket that closes it, with comments dropped and whitespace dropped
//! outside literals. A predicate rustfmt wraps across lines is one unit, so
//! neither spacing nor wrapping hides anything. A Rust source whose
//! comment, literal or unit never closes cannot be read, and is refused
//! whole.
//!
//! The spellings are the ones a Rust or Cargo author writes: the bare
//! predicate under `cfg`, `cfg!`, `cfg_attr`, `not`, `any` and `all`; the
//! `target_os` and `target_family` keys naming Windows and the
//! `target_env` key naming MSVC; on one line, the predicate after a comma
//! on a line that names `cfg`; a Cargo `target.<triple>` key whose triple
//! names Windows, in a Cargo manifest or `.cargo/` configuration; and a
//! runtime check, the Windows name as a string beside
//! `std::env::consts::OS` or `FAMILY`, or compared with `==` or `!=`, or
//! matched with `=>`. A file that is tracked but cannot be read fails the
//! check; one that is tracked and deleted from the working tree is not
//! there to hold anything.
//!
//! Deliberately unread, each for the reason given:
//! - A Windows target triple outside a Cargo manifest or `.cargo/`
//!   configuration: `Cargo.lock` names the `winapi` triple crates a
//!   dependency pulls in, which no author here wrote.
//! - A runtime check whose name and comparison stand on different lines,
//!   or whose name is held in a constant or variable: finding it means
//!   following a value, which a reader of text cannot do.
//! - A `cfg_select!` arm: its predicate stands beside arbitrary code, where
//!   the bare name is an ordinary identifier. The workspace uses none.
//! - An attribute inside a doc comment's example, or in a file that is not
//!   Rust: rustfmt wraps neither, so each is read by line only.

use std::collections::BTreeMap;
use std::ops::RangeInclusive;
use std::path::PathBuf;

#[path = "support/rust_source.rs"]
mod rust_source;
#[path = "support/tracked.rs"]
mod tracked_files;

use rust_source::{skip_literal, units};

/// Paths whose words are fixed when they are written, each with the reason
/// it is not rewritten.
const RECORDS: [(&str, &str); 4] = [
    (
        "docs/decisions/",
        "a decision's text is fixed when it is ruled",
    ),
    ("docs/releases/", "release notes say what shipped"),
    (
        "openspec/changes/archive/",
        "an archived change records what was built",
    ),
    (
        "openspec/changes/2026-09-09-226-session-resumption/tasks.md",
        "a task ledger records the work done before decision 0063",
    ),
];

/// The platform the spellings name, held once so this file spells none of
/// them itself.
const OS: &str = "windows";

/// The target environment only Windows has, held once for the same reason.
const MSVC: &str = "msvc";

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

/// The `key = "value"` predicates that name Windows, squeezed.
fn keyed() -> [String; 3] {
    [
        format!("target_os=\"{OS}\""),
        format!("target_family=\"{OS}\""),
        format!("target_env=\"{MSVC}\""),
    ]
}

/// The Windows conditional a line of the file at `path` spells, if any.
fn spelling(path: &str, line: &str) -> Option<String> {
    let squeezed: String = line.chars().filter(|c| !c.is_whitespace()).collect();
    predicate(&squeezed)
        .or_else(|| runtime(&squeezed))
        .or_else(|| cargo_manifest(path).then(|| target_key(&squeezed))?)
}

/// A cfg predicate naming Windows on one squeezed line.
fn predicate(squeezed: &str) -> Option<String> {
    let anywhere = [
        format!("cfg({OS})"),
        format!("cfg!({OS})"),
        format!("cfg_attr({OS},"),
        format!("not({OS})"),
        format!("any({OS},"),
        format!("all({OS},"),
    ];
    let after_a_comma = [format!(",{OS})"), format!(",{OS},")];
    anywhere
        .into_iter()
        .chain(keyed())
        .find(|needle| squeezed.contains(needle.as_str()))
        .or_else(|| {
            after_a_comma
                .into_iter()
                .find(|needle| squeezed.contains("cfg") && squeezed.contains(needle.as_str()))
        })
}

/// A runtime check against the Windows name on one squeezed line.
fn runtime(squeezed: &str) -> Option<String> {
    let name = format!("\"{OS}\"");
    if !squeezed.contains(name.as_str()) {
        return None;
    }
    ["consts::OS", "consts::FAMILY"]
        .into_iter()
        .find(|constant| squeezed.contains(constant))
        .map(|constant| format!("{constant} {name}"))
        .or_else(|| {
            [
                format!("=={name}"),
                format!("{name}=="),
                format!("!={name}"),
                format!("{name}!="),
                format!("{name}=>"),
            ]
            .into_iter()
            .find(|needle| squeezed.contains(needle.as_str()))
        })
}

/// Whether `path` is a Cargo manifest or Cargo configuration, where a
/// `target.<triple>` key selects a platform.
fn cargo_manifest(path: &str) -> bool {
    path.rsplit('/').next() == Some("Cargo.toml")
        || path.starts_with(".cargo/")
        || path.contains("/.cargo/")
}

/// A `target.<triple>` table or dotted key whose triple is a Windows one.
fn target_key(squeezed: &str) -> Option<String> {
    let unquoted: String = squeezed
        .chars()
        .filter(|c| !matches!(c, '"' | '\''))
        .collect();
    let key = unquoted
        .strip_prefix('[')
        .unwrap_or(&unquoted)
        .strip_prefix("target.")?;
    let triple = key.split(['.', ']', '=']).next()?;
    triple
        .contains(&format!("-{OS}-"))
        .then(|| format!("target.{triple}"))
}

/// A unit's text with whitespace dropped outside literals, twice: `code`
/// keeps each literal's text (a raw string without its `r` and `#`s), and
/// `bare` empties it, so no string reads as a predicate.
fn squeeze(unit: &str) -> Result<(String, String), String> {
    let s: Vec<char> = unit.chars().collect();
    let (mut code, mut bare, mut i) = (String::new(), String::new(), 0);
    while i < s.len() {
        let end = skip_literal(&s, i)?;
        if end != i {
            let literal: String = s[i..end].iter().collect();
            code.push_str(
                literal
                    .trim_start_matches(['b', 'r', '#'])
                    .trim_end_matches('#'),
            );
            bare.push_str("\"\"");
            i = end;
            continue;
        }
        if !s[i].is_whitespace() {
            code.push(s[i]);
            bare.push(s[i]);
        }
        i += 1;
    }
    Ok((code, bare))
}

/// The Windows name standing as a bare predicate, between `(` or `,` and
/// `)` or `,`.
fn bare_predicate(bare: &str) -> Option<String> {
    bare.match_indices(OS).find_map(|(at, _)| {
        let before = bare[..at]
            .chars()
            .next_back()
            .filter(|c| matches!(c, '(' | ','))?;
        let after = bare[at + OS.len()..]
            .chars()
            .next()
            .filter(|c| matches!(c, ')' | ','))?;
        Some(format!("{before}{OS}{after}"))
    })
}

/// The Windows predicate a unit that names `cfg` holds, if any.
fn unit_spelling(unit: &str) -> Result<Option<String>, String> {
    let (code, bare) = squeeze(unit)?;
    if !bare.contains("cfg") {
        return Ok(None);
    }
    Ok(keyed()
        .into_iter()
        .find(|key| code.contains(key.as_str()))
        .or_else(|| bare_predicate(&bare)))
}

/// Each Windows predicate a Rust source's units hold, with the lines the
/// unit spans, or why the source cannot be read.
fn unit_findings(text: &str) -> Result<Vec<(RangeInclusive<usize>, String)>, String> {
    let mut found = Vec::new();
    for (lines, unit) in units(text)? {
        found.extend(unit_spelling(&unit)?.map(|spelled| (lines, spelled)));
    }
    Ok(found)
}

/// Every Windows conditional `text` spells, as `path:line: spelling`, one
/// per line; a unit found by line already is not reported again, and a
/// Rust source that cannot be read is refused whole.
fn findings(path: &str, text: &str) -> Vec<String> {
    if RECORDS.iter().any(|(record, _)| path.starts_with(record)) {
        return Vec::new();
    }
    let units = match path.ends_with(".rs").then(|| unit_findings(text)) {
        Some(Err(why)) => return vec![format!("{path}: cannot be read: {why}")],
        Some(Ok(units)) => units,
        None => Vec::new(),
    };
    let mut found: BTreeMap<usize, String> = text
        .lines()
        .enumerate()
        .filter_map(|(index, line)| spelling(path, line).map(|found| (index + 1, found)))
        .collect();
    for (lines, spelled) in units {
        if found.range(lines.clone()).next().is_none() {
            found.insert(*lines.start(), spelled);
        }
    }
    found
        .into_iter()
        .map(|(line, spelled)| format!("{path}:{line}: {spelled}"))
        .collect()
}

#[test]
fn no_tracked_file_spells_a_windows_conditional() {
    let root = workspace();
    let mut refused = Vec::new();
    for path in tracked_files::tracked(&root, &[]) {
        let bytes = match std::fs::read(root.join(&path)) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => panic!("{path} is tracked and cannot be read: {error}"),
        };
        refused.extend(findings(&path, &String::from_utf8_lossy(&bytes)));
    }
    assert_eq!(
        refused,
        Vec::<String>::new(),
        "decision 0063: the hosts are Linux and macOS, and a Windows conditional \
         is refused outside the records"
    );
}

#[test]
fn every_spelling_is_found_and_a_record_keeps_its_words() {
    let planted = [
        format!("#[cfg({OS})]"),
        format!("    if cfg!( {OS} ) {{"),
        format!("#[cfg_attr({OS}, allow(dead_code))]"),
        format!("#[cfg(not({OS}))]"),
        format!("#[cfg(any({OS}, target_os = \"macos\"))]"),
        format!("#[cfg(all({OS}, test))]"),
        format!("#[cfg(target_os = \"{OS}\")]"),
        format!("#[cfg(target_family = \"{OS}\")]"),
        format!("#[cfg(any(unix, {OS}))]"),
        format!("#[cfg(any(unix, {OS}, test))]"),
        format!("[target.'cfg({OS})'.dependencies]"),
        format!("#[cfg(unix)] // not {OS}, which is not a host"),
        format!("let {OS} = slice.{OS}(2);"),
        format!("run(first, {OS})"),
        format!("#[cfg(target_env = \"{MSVC}\")]"),
        format!("    if std::env::consts::OS == \"{OS}\" {{"),
        format!("    if host != \"{OS}\" {{"),
        format!("        \"{OS}\" => Some(0),"),
        format!("let host = \"{OS}\";"),
    ]
    .join("\n");
    assert_eq!(
        findings("crates/example/src/lib.rs", &planted),
        [
            format!("crates/example/src/lib.rs:1: cfg({OS})"),
            format!("crates/example/src/lib.rs:2: cfg!({OS})"),
            format!("crates/example/src/lib.rs:3: cfg_attr({OS},"),
            format!("crates/example/src/lib.rs:4: not({OS})"),
            format!("crates/example/src/lib.rs:5: any({OS},"),
            format!("crates/example/src/lib.rs:6: all({OS},"),
            format!("crates/example/src/lib.rs:7: target_os=\"{OS}\""),
            format!("crates/example/src/lib.rs:8: target_family=\"{OS}\""),
            format!("crates/example/src/lib.rs:9: ,{OS})"),
            format!("crates/example/src/lib.rs:10: ,{OS},"),
            format!("crates/example/src/lib.rs:11: cfg({OS})"),
            format!("crates/example/src/lib.rs:15: target_env=\"{MSVC}\""),
            format!("crates/example/src/lib.rs:16: consts::OS \"{OS}\""),
            format!("crates/example/src/lib.rs:17: !=\"{OS}\""),
            format!("crates/example/src/lib.rs:18: \"{OS}\"=>"),
        ]
    );
    for (record, _) in RECORDS {
        let path = format!("{record}example.md");
        let path = if record.ends_with('/') {
            path.as_str()
        } else {
            record
        };
        assert_eq!(findings(path, &planted), Vec::<String>::new(), "{path}");
    }
}

#[test]
fn a_windows_target_key_is_read_in_cargo_configuration_only() {
    let planted = [
        format!("[target.x86_64-pc-{OS}-{MSVC}.dependencies]"),
        format!("[target.\"aarch64-pc-{OS}-gnullvm\".dev-dependencies]"),
        format!("target.i686-pc-{OS}-gnu.linker = \"lld\""),
        "[target.x86_64-unknown-linux-gnu.dependencies]".to_string(),
        format!("{OS}-sys = \"0.61\""),
    ]
    .join("\n");
    let triples = |path: &str| {
        [
            format!("{path}:1: target.x86_64-pc-{OS}-{MSVC}"),
            format!("{path}:2: target.aarch64-pc-{OS}-gnullvm"),
            format!("{path}:3: target.i686-pc-{OS}-gnu"),
        ]
    };
    for path in [
        "Cargo.toml",
        "crates/example/Cargo.toml",
        ".cargo/config.toml",
    ] {
        assert_eq!(findings(path, &planted), triples(path), "{path}");
    }
    assert_eq!(findings("Cargo.lock", &planted), Vec::<String>::new());
}

#[test]
fn a_predicate_rustfmt_wraps_is_read_as_one_unit() {
    // Each attribute but the `not` and the split key is rustfmt's (edition
    // 2024) output for its one-line form; those two are wrapped by hand.
    let planted = r#"#[cfg(any(
    feature = "a-long-feature-name-that-pushes-the-line-past-the-width",
    {OS}
))]
fn one() {}
#[cfg(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd",
    {OS}
))]
fn two() {}
#[cfg_attr(
    {OS},
    expect(
        dead_code,
        reason = "a reason long enough that rustfmt must wrap this attribute"
    )
)]
fn three() {}
#[cfg(not(
    {OS}
))]
fn four() {}
#[cfg(not(any(
    feature = "a-long-feature-name-that-pushes-the-line-past-the-width",
    {OS}
)))]
fn five() {}
fn six() -> bool {
    let a_long_binding_name_to_push_the_width = cfg!(any(
        feature = "a-very-long-feature-name-number-one",
        feature = "a-very-long-feature-name-two",
        {OS}
    ));
    a_long_binding_name_to_push_the_width
}
#[cfg(any(
    feature = "a-long-feature-name-that-pushes-the-line-past",
    target_env = "{MSVC}"
))]
fn seven() {}
#[cfg(target_family
    = "{OS}")]
fn eight() {}
const QUOTE: char = '"';
#[cfg(any(
    unix, // a comment's ] closes nothing
    {OS}
))]
fn nine() {}
#[cfg_attr(
    unix,
    doc = "a string that says ({OS}) is no predicate"
)]
fn ten() {}
"#
    .replace("{OS}", OS)
    .replace("{MSVC}", MSVC);
    let path = "crates/example/src/lib.rs";
    assert_eq!(
        findings(path, &planted),
        [
            format!("{path}:1: ,{OS})"),
            format!("{path}:6: ,{OS})"),
            format!("{path}:14: ({OS},"),
            format!("{path}:22: ({OS})"),
            format!("{path}:26: ,{OS})"),
            format!("{path}:32: ,{OS})"),
            format!("{path}:41: target_env=\"{MSVC}\""),
            format!("{path}:44: target_family=\"{OS}\""),
            format!("{path}:48: ,{OS})"),
        ]
    );
}

#[test]
fn a_rust_source_that_cannot_be_read_is_refused() {
    let path = "crates/example/src/lib.rs";
    for (text, why) in [
        (
            "fn f() {}\n#[cfg(any(\n    unix,\n",
            "an unterminated attribute",
        ),
        (
            "let x = cfg!(any(\n    unix,\n",
            "an unterminated cfg! invocation",
        ),
        ("let s = \"never closed;\n", "an unterminated string"),
    ] {
        assert_eq!(
            findings(path, text),
            [format!("{path}: cannot be read: {why}")],
            "{text}"
        );
        assert_eq!(findings("docs/example.md", text), Vec::<String>::new());
    }
}
