//! Agy's example adapter held to its evidence record and to the guide
//! (#1224): each MCP shape names the version its recordings are from, and
//! the guide says what the adapter declares.
//!
//! The citation test binds each MCP sentence to the recordings
//! [`AGY_CITATIONS`] lists for it, each a file of the evidence folder of the
//! sentence's version, named in whole tokens at one place beside its excerpt,
//! where a citation is a token ending in a recording's extension, which must
//! be a file of that folder, or a hyphenated token that is a recording's name
//! in that folder or, naming none, begins one up to a hyphen. Every other
//! token is prose, and the test does not judge prose.

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

/// The endings that make a token a citation, whether or not the file exists.
const CITATION_ENDINGS: [&str; 5] = [".ndjson", ".txt", ".hooklog", ".starts", ".stderr"];

/// `c` may stand within a token.
fn within(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '.' | '_' | '-')
}

/// `token` ends in a recording's extension.
fn file_name(token: &str) -> bool {
    (CITATION_ENDINGS.iter()).any(|ending| token.ends_with(ending))
}

/// The tokens of `text`: each a longest run of letters, digits, `.`, `_`
/// and `-`, with a trailing `.` dropped.
fn tokens(text: &str) -> impl Iterator<Item = &str> {
    (text.split(|c: char| !within(c)))
        .map(|token| token.strip_suffix('.').unwrap_or(token))
        .filter(|token| !token.is_empty())
}

/// `token` is `recording`, or a beginning of it that ends where a hyphen
/// follows: `iso-05` for `iso-05-above-git-root`.
fn names(token: &str, recording: &str) -> bool {
    (recording.strip_prefix(token)).is_some_and(|rest| rest.is_empty() || rest.starts_with('-'))
}

/// An excerpt of a sentence and the recordings it cites beside the claim
/// they show.
type Cited = (&'static str, &'static [&'static str]);

/// The one excerpt of each unboxed sentence that says the terminal sandbox
/// restricted nothing.
const SANDBOX_NOTHING: &[Cited] = &[(
    "on 1.5.13 --sandbox restricted nothing in sandbox-skip-permissions",
    &["sandbox-skip-permissions"],
)];

/// What each sentence of agy's example cites, keyed by its hands and axis
/// (or `carriage`). Both invocations of a hands kind share the entry.
const AGY_CITATIONS: [(&str, &[Cited]); 9] = [
    (
        "carriage",
        &[
            (
                "iso-07-engine-carriage-cold-v14 (cold)",
                &["iso-07-engine-carriage-cold-v14"],
            ),
            (
                "iso-09-engine-carriage-resume-v14 (a resume of it)",
                &["iso-09-engine-carriage-resume-v14"],
            ),
        ],
    ),
    (
        "none ambient",
        &[
            (
                "iso-01, the positive control: all six sentinels started",
                &["iso-01-positive-control"],
            ),
            (
                "iso-05 and iso-06",
                &["iso-05-above-git-root", "iso-06-parent-without-git"],
            ),
            (
                "iso-02's workspace servers started",
                &["iso-02-engine-home-workspace-cwd"],
            ),
            (
                "up to where agy's walk ends (git-root-linked-worktree)",
                &["git-root-linked-worktree"],
            ),
        ],
    ),
    ("none native_write", SANDBOX_NOTHING),
    ("none store_read", SANDBOX_NOTHING),
    (
        "none process_read",
        &[(
            "on 1.5.13 ps -e saw the host's processes under --sandbox in sandbox-skip-permissions",
            &["sandbox-skip-permissions"],
        )],
    ),
    (
        "boxed ambient",
        &[
            (
                "iso-01-positive-control-v14: all six sentinels started",
                &["iso-01-positive-control-v14"],
            ),
            (
                "iso-02-engine-home-workspace-cwd-v14, iso-05-above-git-root-v14, \
                 iso-06-parent-without-git-v14: a scratch HOME shut out the user's",
                &[
                    "iso-02-engine-home-workspace-cwd-v14",
                    "iso-05-above-git-root-v14",
                    "iso-06-parent-without-git-v14",
                ],
            ),
            (
                "iso-07-engine-carriage-cold-v14, box-engine-workspace-tool-v14: the engine's",
                &[
                    "iso-07-engine-carriage-cold-v14",
                    "box-engine-workspace-tool-v14",
                ],
            ),
        ],
    ),
    (
        "boxed native_write",
        &[(
            "box-engine-workspace-tool-v14 and box-other-natives-v14: under the driver's gate, \
             listing call_mcp_tool alone, write_file, edit_file and run_command were refused by \
             name, and the hands tool wrote in the worktree",
            &["box-engine-workspace-tool-v14", "box-other-natives-v14"],
        )],
    ),
    (
        "boxed store_read",
        &[
            (
                "denied to the hands tool (box-store-drive)",
                &["box-store-drive"],
            ),
            (
                "which read it unsealed (box-store-drive-control)",
                &["box-store-drive-control"],
            ),
            (
                "refused by name before either reached the path (box-store)",
                &["box-store"],
            ),
        ],
    ),
    (
        "boxed process_read",
        &[
            (
                "box-reach-v14: the hands tool's command saw 4 entries in /proc with bubblewrap \
                 as pid 1",
                &["box-reach-v14"],
            ),
            (
                "box-engine-workspace-tool-v14: the native shell, run_command, was refused by name",
                &["box-engine-workspace-tool-v14"],
            ),
        ],
    ),
];

/// The entry of [`AGY_CITATIONS`] for `key`.
fn citations(key: &str) -> &'static [Cited] {
    let entry = AGY_CITATIONS.iter().find(|(named, _)| *named == key);
    entry.unwrap_or_else(|| panic!("no citations for {key}")).1
}

/// How a sentence fails its entry of [`AGY_CITATIONS`].
#[derive(Debug, PartialEq)]
enum Miscited<'s> {
    /// The sentence does not hold this excerpt.
    Excerpt(&'static str),
    /// The sentence holds this citation, which names no recording of the
    /// entry.
    Unlisted(&'s str),
}

/// `sentence` holds `excerpt` at a place where its tokens are whole: the
/// excerpt neither begins inside a token nor ends inside one.
fn holds(sentence: &str, excerpt: &str) -> bool {
    let run: Vec<&str> = tokens(excerpt).collect();
    sentence.match_indices(excerpt).any(|(at, _)| {
        let inside =
            excerpt.starts_with(within) && sentence[..at].chars().next_back().is_some_and(within);
        !inside
            && tokens(&sentence[at..])
                .take(run.len())
                .eq(run.iter().copied())
    })
}

/// Agy's evidence record as the adapter's sentences cite it: the name of
/// every file in its folder, whole and before its first dot, and for each
/// 1.5.14 measurement, the fifth and the sixth, the recordings its section
/// is the first to name.
struct AgyRecord {
    files: BTreeSet<String>,
    recordings: BTreeSet<String>,
    fifth: BTreeSet<String>,
    sixth: BTreeSet<String>,
}

impl AgyRecord {
    fn read() -> AgyRecord {
        let evidence = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/evidence/adapters");
        let files: BTreeSet<String> = std::fs::read_dir(evidence.join("agy-cli-1-5"))
            .unwrap()
            .map(Result::unwrap)
            .filter(|entry| entry.file_type().unwrap().is_file())
            .map(|entry| entry.file_name().into_string().unwrap())
            .collect();
        let recordings: BTreeSet<String> = (files.iter())
            .map(|name| name.split('.').next().unwrap().to_string())
            .collect();
        let named = |text: &str| -> BTreeSet<String> {
            (tokens(text).filter_map(|token| token.split('.').next()))
                .filter(|stem| recordings.contains(*stem))
                .map(str::to_string)
                .collect()
        };
        let record = std::fs::read_to_string(evidence.join("agy-cli-1-5.md")).unwrap();
        let (earlier, fifth) = record
            .split_once("\n## Fifth measurement, on 1.5.14")
            .unwrap();
        let (fifth, sixth) = fifth
            .split_once("\n## Sixth measurement, on 1.5.14")
            .unwrap();
        let (sixth, _) = sixth.split_once("\n## Not measured").unwrap();
        let earlier = named(earlier);
        let fifth = &named(fifth) - &earlier;
        let sixth = &(&named(sixth) - &fifth) - &earlier;
        AgyRecord {
            files,
            recordings,
            fifth,
            sixth,
        }
    }

    /// `token` is a citation: it ends in a recording's extension, or it is
    /// hyphenated and names a file of the folder.
    fn cites(&self, token: &str) -> bool {
        file_name(token)
            || token.contains('-') && (self.recordings.iter()).any(|name| names(token, name))
    }

    /// The citation `token` names `recording`: a file name is a file of the
    /// folder whose name before the first dot is that recording, and any
    /// other citation is that recording or, when it is no file's name,
    /// begins it.
    fn resolves(&self, token: &str, recording: &str) -> bool {
        if file_name(token) {
            self.files.contains(token) && token.split('.').next() == Some(recording)
        } else if self.recordings.contains(token) {
            token == recording
        } else {
            names(token, recording)
        }
    }

    /// `sentence` holds every excerpt of `key`'s entry, whole tokens at one
    /// place, each of which must name its recordings, and every citation it
    /// holds names a recording of the entry.
    fn miscited<'s>(&self, key: &str, sentence: &'s str) -> Result<(), Miscited<'s>> {
        let cited = citations(key);
        for &(excerpt, recordings) in cited {
            if !holds(sentence, excerpt) {
                return Err(Miscited::Excerpt(excerpt));
            }
            let run: Vec<&str> = tokens(excerpt).collect();
            let named = |name: &&str| run.iter().any(|t| self.cites(t) && self.resolves(t, name));
            assert!(
                recordings.iter().all(named),
                "{key}: {excerpt:?} names {recordings:?}"
            );
        }
        let listed = cited.iter().flat_map(|(_, recordings)| recordings.iter());
        match tokens(sentence)
            .filter(|token| self.cites(token))
            .find(|token| !listed.clone().any(|name| self.resolves(token, name)))
        {
            Some(token) => Err(Miscited::Unlisted(token)),
            None => Ok(()),
        }
    }

    /// The first of `recordings` that is no file of the folder, or not of
    /// `version`: a 1.5.14 recording is one the measurement `sentence` names
    /// is the first to name, and a 1.5.13 recording one neither 1.5.14
    /// measurement is.
    fn unrecorded(
        &self,
        version: &str,
        sentence: &str,
        recordings: &[&'static str],
    ) -> Option<&'static str> {
        let measurement = if sentence.contains("Fifth measurement") {
            Some(&self.fifth)
        } else if sentence.contains("Sixth measurement") {
            Some(&self.sixth)
        } else {
            None
        };
        let of_version = |name: &str| match version {
            "1.5.14" => measurement.is_some_and(|named| named.contains(name)),
            "1.5.13" => !self.fifth.contains(name) && !self.sixth.contains(name),
            other => panic!("agy's record holds no version {other}"),
        };
        (recordings.iter().copied())
            .find(|name| !self.recordings.contains(*name) || !of_version(name))
    }

    /// `sentence`, of a shape measured on `version`, cites what `key`'s
    /// entry lists, and the entry lists recordings of that version.
    fn assert_cited(&self, key: &str, version: &str, sentence: &str) {
        assert_eq!(self.miscited(key, sentence), Ok(()), "{key}: {sentence}");
        let listed: Vec<&str> = (citations(key).iter())
            .flat_map(|(_, recordings)| recordings.iter().copied())
            .collect();
        let unrecorded = self.unrecorded(version, sentence, &listed);
        assert_eq!(unrecorded, None, "{key} on {version}: {sentence}");
    }
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
    let summary = format!("without hands on {without_hands} and boxed on {},", boxed.0);
    assert!(joined.contains(&summary), "{summary}");
    let store = format!("Store reads are measured boxed, on {},", boxed.0);
    assert_eq!(joined.contains(&store), boxed.1, "{store}");
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
        assert!(measured.contains(&version), "{version}");
    }
    assert_eq!(
        (
            holds_all(measured, &GUIDE_STORE_ROW),
            unmeasured_boxed.contains("store read")
        ),
        (boxed.1, !boxed.1)
    );
}

/// E1 and E2 of #1224's fourth round, bound by its fifth: every sentence of
/// Agy's example, the carriage's and each axis of each shape, cites what
/// [`AGY_CITATIONS`] lists for it, each recording a file of the version the
/// shape declares and, on 1.5.14, of the measurement the sentence names; the
/// boxed shapes measured on 1.5.14, each of their four axes measured; the
/// shapes without hands still on 1.5.13, their ambient axis alone measured;
/// and the guide says what the adapter does.
#[test]
fn each_agy_shape_names_the_version_its_recordings_are_from() {
    let record = AgyRecord::read();
    let (_dir, adapters) = agy_example();
    let agy = adapters.adapter("agy").unwrap();
    let McpSupport::Declared { carriage, shapes } = &agy.mcp else {
        panic!("agy declares its MCP facts: {:?}", agy.mcp);
    };
    let McpAxis::Measured { evidence } = carriage else {
        panic!("{carriage:?}");
    };
    assert!(evidence.contains("Antigravity CLI 1.5.14"), "{evidence}");
    record.assert_cited("carriage", "1.5.14", evidence);
    let mut judged = vec!["carriage".to_string()];
    let mut versions = Vec::new();
    let mut measured = Vec::new();
    for (shape, measurement) in shapes {
        let isolation = &measurement.isolation;
        let axes = [
            ("ambient", &isolation.ambient),
            ("native_write", &isolation.native_write),
            ("store_read", &isolation.store_read),
            ("process_read", &isolation.process_read),
        ];
        let hands = if shape.hands == McpHands::Boxed {
            "boxed"
        } else {
            "none"
        };
        for (name, axis) in axes {
            let sentence = match axis {
                McpAxis::Measured { evidence } => {
                    measured.push(format!("{shape} {name}"));
                    evidence
                }
                McpAxis::Unmeasured(McpUnmeasured::Declared(why)) => why,
                other => panic!("{shape}: {other:?}"),
            };
            let key = format!("{hands} {name}");
            record.assert_cited(&key, &measurement.version, sentence);
            judged.push(key);
        }
        let store_measured = matches!(isolation.store_read, McpAxis::Measured { .. });
        versions.push((
            shape.to_string(),
            measurement.version.as_str(),
            store_measured,
        ));
    }
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
    assert_eq!(measured, expected_measured);
    // The carriage and the four axes of each of the four shapes, and every
    // entry of the table judged.
    assert_eq!(judged.len(), 17);
    let judged: BTreeSet<&str> = judged.iter().map(String::as_str).collect();
    let listed: BTreeSet<&str> = AGY_CITATIONS.iter().map(|(key, _)| *key).collect();
    assert_eq!(judged, listed);
    assert_the_guide_agrees((versions[0].1, versions[0].2), versions[1].1);
}

/// F1 of #1224's fifth round: on copies of shipped sentences, a file name
/// the entry does not list fails wherever it is appended, a recording
/// swapped for another of the same measurement, misspelt or prefixed fails,
/// even beside a second copy of its excerpt, a recording's exact name is not
/// read as a short form of a listed one, a listed recording's file name
/// passes and one of no file fails, a short form passes bare and fails as a
/// file name, a hyphenated word of prose passes, and a recording of no file,
/// of the other version or of the other measurement is refused.
#[test]
fn an_agy_citation_is_a_recording_s_file_name_and_prose_is_not_judged() {
    let record = AgyRecord::read();
    let (_dir, adapters) = agy_example();
    let boxed = adapters.adapter("agy").unwrap().mcp.isolation(&McpShape {
        invocation: McpInvocation::Cold,
        hands: McpHands::Boxed,
        host: McpHost::Linux,
    });
    let unboxed = adapters.adapter("agy").unwrap().mcp.isolation(&McpShape {
        invocation: McpInvocation::Cold,
        hands: McpHands::NoHands,
        host: McpHost::Linux,
    });
    let (
        McpAxis::Measured { evidence: process },
        McpAxis::Measured { evidence: store },
        McpAxis::Measured { evidence: ambient },
    ) = (&boxed.process_read, &boxed.store_read, &unboxed.ambient)
    else {
        panic!("{boxed:?} {unboxed:?}");
    };
    let key = "boxed process_read";
    let [reach, _] = citations(key) else {
        panic!("{key} has two excerpts");
    };
    let appended = |tail: &str| format!("{process} {tail}");
    assert_eq!(
        [
            record.miscited(key, process),
            record.miscited(key, &appended("version.txt (1.5.13)")),
            record.miscited(key, &appended("box_reach_v14.ndjson (missing)")),
            record.miscited(
                key,
                &process.replace("box-reach-v14", "box-other-natives-v14")
            ),
            record.miscited(key, &process.replace("box-reach-v14", "box-reach-v41")),
            record.miscited(key, &process.replace("box-reach-v14", "fake-box-reach-v14")),
            record.miscited(key, &appended("and box-reach")),
            record.miscited(key, &appended("(box-reach-v14.ndjson)")),
            record.miscited(key, &appended("before the pre-tool hook")),
            record.miscited(key, &appended("(box-reach-v14.stderr)")),
            record.miscited(
                key,
                &format!(
                    "{} box-reach-v14. the hands tool's command saw 4 entries in /proc with \
                     bubblewrap as pid 1",
                    process.replace("box-reach-v14:", "fake-box-reach-v14:")
                )
            ),
            record.miscited("none ambient", ambient),
            record.miscited("none ambient", &format!("{ambient} (iso-05.ndjson)")),
        ],
        [
            Ok(()),
            Err(Miscited::Unlisted("version.txt")),
            Err(Miscited::Unlisted("box_reach_v14.ndjson")),
            Err(Miscited::Excerpt(reach.0)),
            Err(Miscited::Excerpt(reach.0)),
            Err(Miscited::Excerpt(reach.0)),
            Err(Miscited::Unlisted("box-reach")),
            Ok(()),
            Ok(()),
            Err(Miscited::Unlisted("box-reach-v14.stderr")),
            Err(Miscited::Excerpt(reach.0)),
            Ok(()),
            Err(Miscited::Unlisted("iso-05.ndjson")),
        ]
    );
    let elsewhere = store.replace("Fifth measurement", "Sixth measurement");
    assert_eq!(
        [
            record.unrecorded("1.5.14", store, &["box-store", "box-reach-v41"]),
            record.unrecorded("1.5.14", store, &["box-store", "git-root-linked-worktree"]),
            record.unrecorded("1.5.13", store, &["git-root-linked-worktree", "box-store"]),
            record.unrecorded("1.5.14", &elsewhere, &["box-store"]),
        ],
        [
            Some("box-reach-v41"),
            Some("git-root-linked-worktree"),
            Some("box-store"),
            Some("box-store"),
        ]
    );
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
