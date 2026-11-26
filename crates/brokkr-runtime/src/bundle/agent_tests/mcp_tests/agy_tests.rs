//! Agy's example adapter held to its evidence record and to the guide
//! (#1224): each MCP shape names the version its recordings are from, and
//! the guide says what the adapter declares.
//!
//! The citation test binds each row of [`AGY_CITATIONS`]: its file is in
//! agy's evidence folder, its token stands whole in its sentence, and its
//! version is the `measured_on.version` of the sentence's shape, where the
//! sentence is of a shape, and 1.5.14 for a file whose name ends `-v14`. It
//! does not detect a citation the table does not list, and it does not
//! judge prose. A version written in a sentence's prose is not judged.

use super::*;

/// Agy's example adapter, loaded from a copy of the shipped file, and the
/// directory that holds the copy.
fn agy_example() -> (tempfile::TempDir, Adapters) {
    let dir = tempfile::tempdir().unwrap();
    let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/examples/adapters");
    std::fs::copy(example.join("agy.json"), dir.path().join("agy.json")).unwrap();
    let adapters = Adapters::load(dir.path()).unwrap();
    (dir, adapters)
}

/// One citation of a sentence: the token exactly as the sentence writes
/// it, the evidence file it stands for, and the agy version that file was
/// recorded on.
type Row = (&'static str, &'static str, &'static str);

/// The rows of the unboxed sentences that say the terminal sandbox
/// restricted nothing, or hid no process.
const SANDBOX: &[Row] = &[(
    "sandbox-skip-permissions",
    "sandbox-skip-permissions.ndjson",
    "1.5.13",
)];

/// What each sentence of agy's example cites, keyed by its hands and axis
/// (or `carriage`). Both invocations of a hands kind share the entry.
const AGY_CITATIONS: [(&str, &[Row]); 9] = [
    (
        "carriage",
        &[
            (
                "iso-07-engine-carriage-cold-v14",
                "iso-07-engine-carriage-cold-v14.ndjson",
                "1.5.14",
            ),
            (
                "iso-09-engine-carriage-resume-v14",
                "iso-09-engine-carriage-resume-v14.ndjson",
                "1.5.14",
            ),
        ],
    ),
    (
        "none ambient",
        &[
            ("iso-01", "iso-01-positive-control.ndjson", "1.5.13"),
            (
                "iso-02",
                "iso-02-engine-home-workspace-cwd.ndjson",
                "1.5.13",
            ),
            ("iso-05", "iso-05-above-git-root.ndjson", "1.5.13"),
            ("iso-06", "iso-06-parent-without-git.ndjson", "1.5.13"),
            (
                "git-root-linked-worktree",
                "git-root-linked-worktree.ndjson",
                "1.5.13",
            ),
        ],
    ),
    ("none native_write", SANDBOX),
    ("none store_read", SANDBOX),
    ("none process_read", SANDBOX),
    (
        "boxed ambient",
        &[
            (
                "iso-01-positive-control-v14",
                "iso-01-positive-control-v14.ndjson",
                "1.5.14",
            ),
            (
                "iso-02-engine-home-workspace-cwd-v14",
                "iso-02-engine-home-workspace-cwd-v14.ndjson",
                "1.5.14",
            ),
            (
                "iso-05-above-git-root-v14",
                "iso-05-above-git-root-v14.ndjson",
                "1.5.14",
            ),
            (
                "iso-06-parent-without-git-v14",
                "iso-06-parent-without-git-v14.ndjson",
                "1.5.14",
            ),
            (
                "iso-07-engine-carriage-cold-v14",
                "iso-07-engine-carriage-cold-v14.ndjson",
                "1.5.14",
            ),
            (
                "box-engine-workspace-tool-v14",
                "box-engine-workspace-tool-v14.ndjson",
                "1.5.14",
            ),
        ],
    ),
    (
        "boxed native_write",
        &[
            (
                "box-engine-workspace-tool-v14",
                "box-engine-workspace-tool-v14.ndjson",
                "1.5.14",
            ),
            (
                "box-other-natives-v14",
                "box-other-natives-v14.ndjson",
                "1.5.14",
            ),
        ],
    ),
    (
        "boxed store_read",
        &[
            ("box-store-drive", "box-store-drive.txt", "1.5.14"),
            (
                "box-store-drive-control",
                "box-store-drive-control.txt",
                "1.5.14",
            ),
            ("box-store", "box-store.ndjson", "1.5.14"),
        ],
    ),
    (
        "boxed process_read",
        &[
            ("box-reach-v14", "box-reach-v14.ndjson", "1.5.14"),
            (
                "box-engine-workspace-tool-v14",
                "box-engine-workspace-tool-v14.ndjson",
                "1.5.14",
            ),
        ],
    ),
];

/// The rows of [`AGY_CITATIONS`] for `key`.
fn rows(key: &str) -> &'static [Row] {
    let entry = AGY_CITATIONS.iter().find(|(named, _)| *named == key);
    entry.unwrap_or_else(|| panic!("no citations for {key}")).1
}

/// The name of every file in agy's evidence folder.
fn evidence_files() -> BTreeSet<String> {
    let folder =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/evidence/adapters/agy-cli-1-5");
    (std::fs::read_dir(folder).unwrap())
        .map(Result::unwrap)
        .filter(|entry| entry.file_type().unwrap().is_file())
        .map(|entry| entry.file_name().into_string().unwrap())
        .collect()
}

/// One sentence of agy's example: its key in [`AGY_CITATIONS`], the shape
/// and axis it is of (or `carriage`), the version its shape was measured
/// on (none for the carriage, which is of no shape), its text, and whether
/// it says the axis is measured.
struct Sentence<'a> {
    key: String,
    name: String,
    version: Option<&'a str>,
    text: &'a str,
    measured: bool,
}

/// The carriage's sentence, of no shape, and every axis sentence of every
/// shape, on its shape's `measured_on.version`.
fn sentences(mcp: &McpSupport) -> Vec<Sentence<'_>> {
    let McpSupport::Declared { carriage, shapes } = mcp else {
        panic!("agy declares its MCP facts: {mcp:?}");
    };
    let McpAxis::Measured { evidence } = carriage else {
        panic!("{carriage:?}");
    };
    let mut all = vec![Sentence {
        key: "carriage".into(),
        name: "carriage".into(),
        version: None,
        text: evidence,
        measured: true,
    }];
    for (shape, measurement) in shapes {
        let isolation = &measurement.isolation;
        let hands = match shape.hands == McpHands::Boxed {
            true => "boxed",
            false => "none",
        };
        for (axis, value) in [
            ("ambient", &isolation.ambient),
            ("native_write", &isolation.native_write),
            ("store_read", &isolation.store_read),
            ("process_read", &isolation.process_read),
        ] {
            let (text, measured) = match value {
                McpAxis::Measured { evidence } => (evidence.as_str(), true),
                McpAxis::Unmeasured(McpUnmeasured::Declared(why)) => (why.as_str(), false),
                other => panic!("{shape}: {other:?}"),
            };
            all.push(Sentence {
                key: format!("{hands} {axis}"),
                name: format!("{shape} {axis}"),
                version: Some(&measurement.version),
                text,
                measured,
            });
        }
    }
    all
}

/// `token` stands in `sentence` as a whole token: bounded by anything that
/// is not a letter, a digit, `.`, `_` or `-`, where a `.` followed by
/// whitespace or the sentence's end ends the sentence or a clause and is no
/// part of it. A `.` followed by anything else continues the token.
fn whole(sentence: &str, token: &str) -> bool {
    let within = |c: char| c.is_alphanumeric() || matches!(c, '.' | '_' | '-');
    sentence.match_indices(token).any(|(at, _)| {
        let before = sentence[..at].chars().next_back();
        let mut after = sentence[at + token.len()..].chars();
        let ends = match after.next() {
            Some('.') => after.next().is_none_or(char::is_whitespace),
            Some(c) => !within(c),
            None => true,
        };
        !before.is_some_and(within) && ends
    })
}

/// `sentence` with `text` in its place.
fn reworded<'a>(sentence: &Sentence<'a>, text: &'a str) -> Sentence<'a> {
    Sentence {
        text,
        key: sentence.key.clone(),
        name: sentence.name.clone(),
        ..*sentence
    }
}

/// How a row fails its sentence.
#[derive(Debug, PartialEq)]
enum Uncited {
    /// (a) The row's file is not in the evidence folder.
    Unrecorded(&'static str),
    /// (b) The row's token does not stand whole in the sentence.
    Unwritten(&'static str),
    /// (c) The row's version is not its sentence's shape's, or a `-v14`
    /// file's row is not 1.5.14.
    Misversioned(&'static str),
}

/// The first of `rows` that fails `sentence`, by (a), (b) and (c) in turn.
fn uncited(rows: &[Row], sentence: &Sentence<'_>, files: &BTreeSet<String>) -> Result<(), Uncited> {
    for &(token, file, version) in rows {
        if !files.contains(file) {
            return Err(Uncited::Unrecorded(file));
        }
        if !whole(sentence.text, token) {
            return Err(Uncited::Unwritten(token));
        }
        let stem = Path::new(file).file_stem().and_then(|stem| stem.to_str());
        let v14 = stem.is_some_and(|stem| stem.ends_with("-v14"));
        let unshaped = sentence.version.is_some_and(|shaped| shaped != version);
        if unshaped || (v14 && version != "1.5.14") {
            return Err(Uncited::Misversioned(file));
        }
    }
    Ok(())
}

/// The cells of the guide's table row that begins with `start`.
fn guide_row<'g>(guide: &'g str, start: &str) -> Vec<&'g str> {
    let row = guide.lines().find(|line| line.starts_with(start));
    row.unwrap_or_else(|| panic!("no row {start}"))
        .split(" | ")
        .collect()
}

/// The guide's two denials of a boxed agy store read in the boxed seat's
/// paragraph, each beside the recordings that show it: the seal, its
/// control, and the driver's native gate.
const GUIDE_STORE_PARAGRAPH: [&str; 3] = [
    "with the store sealed at its production path, the hands tool was denied it \
     (`box-store-drive`)",
    "and with no seal it read it (`box-store-drive-control`)",
    "the native `read_file` and `run_command` were refused by name before either reached \
     the path (`box-store`), which shows the gate, not the seal",
];

/// The same two denials in the guide's SI2 row, which points at the
/// paragraph.
const GUIDE_STORE_ROW: [&str; 3] = [
    "kept apart as the [boxed seat's paragraph](#a-boxed-agy-seat) keeps them",
    "the box's seal denied a store at its production path to the hands tool, which read it \
     with no seal (`box-store-drive`, `box-store-drive-control`)",
    "the driver's gate refused the native `read_file` and `run_command` by name before \
     either reached the path (`box-store`), which shows the gate, not the seal",
];

/// The guide says of agy's shapes what the adapter declares, given the
/// version and store measurement of the cold boxed shape and the version of
/// the cold shape without hands: its summary, its version table, its boxed
/// seat's store paragraph and its SI2 row.
fn assert_the_guide_agrees(boxed: (&str, bool), without_hands: &str) {
    let guide =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/guides/provider-adapters.md");
    let guide = std::fs::read_to_string(guide).unwrap();
    let joined = guide.replace('\n', " ");
    // Each version is found by the one boundary rule, [`whole`], with the
    // comma that follows it in the guide.
    let summary = format!("without hands on {without_hands} and boxed on {},", boxed.0);
    assert!(whole(&joined, &summary), "{summary}");
    let store = format!("Store reads are measured boxed, on {},", boxed.0);
    assert_eq!(whole(&joined, &store), boxed.1, "{store}");
    let holds_all = |text: &str, claims: &[&str]| claims.iter().all(|claim| text.contains(claim));
    assert_eq!(holds_all(&joined, &GUIDE_STORE_PARAGRAPH), boxed.1);
    assert!(guide.contains("\n#### A boxed agy seat\n"));
    let last = |start| guide_row(&guide, start).last().copied();
    assert_eq!(
        [
            last("| The boxed shapes' four axes"),
            last("| The shapes without hands")
        ],
        [
            Some(&*format!("{} |", boxed.0)),
            Some(&*format!("{without_hands} |"))
        ]
    );
    let [_, measured, unmeasured] = guide_row(&guide, "| `agy` (example) |")[..] else {
        panic!("the guide's SI2 row for agy has three cells");
    };
    let (_, unmeasured_boxed) = unmeasured.split_once("Boxed:").unwrap();
    for version in [
        format!("Cold and replacement without hands: Antigravity CLI {without_hands},"),
        format!("Cold and replacement, boxed: Antigravity CLI {},", boxed.0),
    ] {
        assert!(whole(measured, &version), "{version}");
    }
    assert_eq!(
        (
            holds_all(measured, &GUIDE_STORE_ROW),
            unmeasured_boxed.contains("store read")
        ),
        (boxed.1, !boxed.1)
    );
}

/// E1 and E2 of #1224's fourth round, cut down to what binds (#1224): every
/// sentence of agy's example, the carriage's and each axis of each shape,
/// holds each row [`AGY_CITATIONS`] lists for it by (a), (b) and (c); a
/// misspelt token, a file that does not exist, a changed shape version and
/// a carriage row whose `-v14` file is not on 1.5.14 each fail, as do a token continued past a `.` and a token led by another
/// word, which a substring check passes; the boxed shapes are measured on 1.5.14, each of their four
/// axes measured; the shapes without hands are still on 1.5.13, their
/// ambient axis alone measured; and the guide says what the adapter does.
#[test]
fn each_agy_shape_names_the_version_its_recordings_are_from() {
    let files = evidence_files();
    let (_dir, adapters) = agy_example();
    let agy = adapters.adapter("agy").unwrap();
    let all = sentences(&agy.mcp);
    for sentence in &all {
        let cited = uncited(rows(&sentence.key), sentence, &files);
        assert_eq!(cited, Ok(()), "{}: {}", sentence.name, sentence.text);
    }
    // The three negatives, one for each of (b), (a) and (c).
    let process = (all.iter())
        .find(|sentence| sentence.key == "boxed process_read")
        .unwrap();
    let misspelt = process.text.replace("box-reach-v14", "box-reach-v41");
    let misspelt = reworded(process, &misspelt);
    let missing: &[Row] = &[("box-reach-v14", "box-reach-v14.stderr", "1.5.14")];
    assert_eq!(
        [
            uncited(rows(&process.key), &misspelt, &files),
            uncited(missing, process, &files),
        ],
        [
            Err(Uncited::Unwritten("box-reach-v14")),
            Err(Uncited::Unrecorded("box-reach-v14.stderr")),
        ]
    );
    // The two boundary negatives: each passes a plain substring check, so
    // only the boundary of [`whole`] fails it.
    for token in ["box-reach-v14./extra", "fake-box-reach-v14"] {
        let text = process.text.replace("box-reach-v14", token);
        assert!(text.contains("box-reach-v14"), "{text}");
        let cited = uncited(rows(&process.key), &reworded(process, &text), &files);
        assert_eq!(cited, Err(Uncited::Unwritten("box-reach-v14")), "{token}");
    }
    let mut changed = agy.clone();
    if let McpSupport::Declared { shapes, .. } = &mut changed.mcp {
        for measurement in shapes.values_mut() {
            measurement.version = "1.5.13".into();
        }
    }
    let refused = (sentences(&changed.mcp).iter())
        .find(|sentence| sentence.key == "boxed process_read")
        .map(|sentence| uncited(rows(&sentence.key), sentence, &files));
    let misversioned = Err(Uncited::Misversioned("box-reach-v14.ndjson"));
    assert_eq!(refused, Some(misversioned));
    // The carriage is of no shape, so only its file's `-v14` name judges
    // the version of its row.
    let carried: &[Row] = &[(
        "iso-07-engine-carriage-cold-v14",
        "iso-07-engine-carriage-cold-v14.ndjson",
        "1.5.13",
    )];
    let misversioned = Uncited::Misversioned("iso-07-engine-carriage-cold-v14.ndjson");
    assert_eq!(uncited(carried, &all[0], &files), Err(misversioned));
    let McpSupport::Declared { shapes, .. } = &agy.mcp else {
        unreachable!("sentences read the declared facts");
    };
    let versions: Vec<(String, &str, bool)> = (shapes.iter())
        .map(|(shape, measurement)| {
            let store = matches!(measurement.isolation.store_read, McpAxis::Measured { .. });
            (shape.to_string(), measurement.version.as_str(), store)
        })
        .collect();
    let shape = |invocation, hands, version, store| {
        let host = McpHost::Linux;
        let shape = McpShape {
            invocation,
            hands,
            host,
        };
        (shape.to_string(), version, store)
    };
    let expected = [
        shape(McpInvocation::Cold, McpHands::Boxed, "1.5.14", true),
        shape(McpInvocation::Cold, McpHands::NoHands, "1.5.13", false),
        shape(McpInvocation::Replacement, McpHands::Boxed, "1.5.14", true),
        shape(
            McpInvocation::Replacement,
            McpHands::NoHands,
            "1.5.13",
            false,
        ),
    ];
    assert_eq!(versions, expected);
    // Each axis of a boxed shape is measured, and only the ambient axis of a
    // shape without hands.
    let boxed = ["ambient", "native_write", "store_read", "process_read"];
    let expected_measured: Vec<String> = (expected.iter())
        .zip([&boxed[..], &boxed[..1], &boxed[..], &boxed[..1]])
        .flat_map(|((shape, _, _), axes)| axes.iter().map(move |axis| format!("{shape} {axis}")))
        .collect();
    let measured: Vec<&str> = (all.iter().skip(1))
        .filter(|sentence| sentence.measured)
        .map(|sentence| sentence.name.as_str())
        .collect();
    assert_eq!(measured, expected_measured);
    // The carriage and the four axes of each of the four shapes, and every
    // entry of the table judged.
    assert_eq!(all.len(), 17);
    let judged: BTreeSet<&str> = all.iter().map(|sentence| sentence.key.as_str()).collect();
    let listed: BTreeSet<&str> = AGY_CITATIONS.iter().map(|(key, _)| *key).collect();
    assert_eq!(judged, listed);
    assert_the_guide_agrees((versions[0].1, versions[0].2), versions[1].1);
}

/// E2 of #1224's fourth round: Agy's boxed shapes declare store read
/// measured, so a boxed agy seat whose MCP holding binds a secret passes
/// the MCP site's store-read check, on the cold shape and the replacement
/// alike; the same adapter with that axis unmeasured refuses it there. The
/// test stops at `McpSite::contribute`: it does not claim the holding is
/// admitted past the sealed resolver's later checks.
#[cfg(target_os = "linux")]
#[test]
fn the_agy_examples_boxed_store_read_is_measured_and_a_secret_passes_the_store_read_check() {
    use crate::capabilities::tests::manifest::holding_outcome;
    use crate::capabilities::{Authority, Outcome, SiteAsks};
    let (dir, adapters) = agy_example();
    let agy = adapters.adapter("agy").unwrap();
    let mut unsealed = agy.clone();
    if let McpSupport::Declared { shapes, .. } = &mut unsealed.mcp {
        for measurement in shapes.values_mut() {
            measurement.isolation.store_read = McpAxis::Unmeasured(McpUnmeasured::Absent);
        }
    }
    let mut holding = mcp_holding("library-docs");
    if let crate::capabilities::Implementation::Mcp { secrets, .. } = &mut holding.implementation {
        secrets.push("DOCS_TOKEN".to_string());
    }
    // One agy candidate holding `library-docs` alone, as the resolver
    // seals it past the fence.
    let outcome = Outcome {
        provider: "agy".into(),
        harness: "agy".into(),
        model: Some("agy-neon".into()),
        ..holding_outcome(holding, "synthetic")
    };
    let authority = Authority::nothing("private", dir.path());
    let asks = SiteAsks::at(SeatClass::Work, "research", None, None).unwrap();
    for invocation in [McpInvocation::Cold, McpInvocation::Replacement] {
        let (shape, ambient) = entry(invocation, McpHands::Boxed, LEGACY);
        let store_read = agy.mcp.isolation(&shape).store_read;
        let McpAxis::Measured { evidence } = &store_read else {
            panic!("{shape}: {store_read:?}");
        };
        assert!(evidence.contains("(box-store-drive-control)"), "{evidence}");
        let judge = |adapter| {
            let site = mcp::McpSite {
                intent: Some(McpIntent::Hands),
                adapter: Some(adapter),
                strict: vec![(shape.clone(), ambient.clone())],
                networked: false,
            };
            let mut judged = outcome.clone();
            site.contribute(authority.who(&asks), &asks.asks, &mut judged)
        };
        let refused = mcp::Unservable::SecretsUnprotected("library-docs".into());
        assert_eq!([judge(agy), judge(&unsealed)], [Ok(()), Err(refused)]);
    }
}

/// The provider-adapters guide's agy model table matches every alias in
/// `docs/examples/adapters/agy.json` and records the measured efforts for each
/// (#1382).
#[test]
fn the_guide_table_names_every_agy_model_and_its_measured_efforts() {
    let (_dir, adapters) = agy_example();
    let agy = adapters.adapter("agy").unwrap();
    let guide =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/guides/provider-adapters.md");
    let guide = std::fs::read_to_string(guide).unwrap();
    let expected = [
        ("agy-neon", "`low`, `medium`, `high`, `xhigh`"),
        ("agy-carbon", "`low`, `medium`, `high`"),
        ("agy-hydrogen", "`low`, `medium`, `high`, `xhigh`"),
        ("agy-argon", "`low`, `medium`, `high`"),
        ("agy-claude-opus", "`low`, `medium`, `high`"),
        ("agy-claude-sonnet", "`low`, `medium`, `high`"),
    ];
    assert_eq!(agy.models.len(), expected.len());
    for (alias, efforts) in expected {
        let slug = agy
            .models
            .get(alias)
            .unwrap_or_else(|| panic!("missing adapter alias {alias}"));
        let start = format!("| `{alias}` |");
        let mut rows = guide.lines().filter(|line| line.starts_with(&start));
        let row: Vec<&str> = rows
            .next()
            .unwrap_or_else(|| panic!("no row {start}"))
            .split(" | ")
            .collect();
        assert_eq!(rows.next(), None, "duplicate row {start}");
        assert_eq!(
            row,
            [
                format!("| `{alias}`"),
                format!("`{slug}`"),
                format!("{efforts} |"),
            ],
            "{alias}"
        );
    }
    let table_rows = guide
        .lines()
        .skip_while(|line| *line != "| Alias | Agy model | Efforts |")
        .skip(2)
        .take_while(|line| line.starts_with('|'))
        .count();
    assert_eq!(
        table_rows,
        expected.len(),
        "unexpected row count in agy model table"
    );
}
