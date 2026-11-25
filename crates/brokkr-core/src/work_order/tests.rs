use serde_json::{json, Value};

use super::*;

macro_rules! fixture {
    ($file:literal) => {
        include_str!(concat!("../../../../fixtures/work-order/", $file))
    };
}

macro_rules! tracker {
    ($name:literal) => {
        include_str!(concat!("../../../brokkr-cli/trackers/", $name, ".json"))
    };
}

const ASSIGNED: &str = fixture!("complete.md");
const DRAFT: &str = fixture!("draft.md");
const NONCANONICAL: &str = fixture!("noncanonical.md");
const UNKNOWN_KEY: &str = fixture!("unknown-key.md");
const UNCLASSIFIED: &str = fixture!("unclassified.md");
const OFF_HOST: &str = fixture!("off-host.md");

const DEFINITIONS: [(&str, &str, &str); 6] = [
    (
        "github",
        tracker!("github"),
        "https://github.com/project/repo/issues/1",
    ),
    (
        "gitlab",
        tracker!("gitlab"),
        "https://gitlab.com/project/repo/-/issues/1",
    ),
    (
        "azure-devops",
        tracker!("azure-devops"),
        "https://dev.azure.com/project/_workitems/edit/1",
    ),
    (
        "linear",
        tracker!("linear"),
        "https://linear.app/project/issue/TEAM-1",
    ),
    (
        "jira",
        tracker!("jira"),
        "https://project.atlassian.net/browse/TEAM-1",
    ),
    (
        "zenhub",
        tracker!("zenhub"),
        "https://app.zenhub.com/workspaces/project/issues/1",
    ),
];

fn trackers() -> Trackers {
    Trackers::from_definitions(DEFINITIONS.map(|(_, definition, _)| definition)).unwrap()
}

fn assert_refused(document: &str, trackers: &Trackers, expected: Refusal, message: &str) {
    let error = WorkOrder::parse(document, trackers).unwrap_err();
    assert_eq!(error.0, expected, "{document}");
    assert_eq!(error.to_string(), message);
}

fn draft_with(field: &str, value: Value) -> String {
    let (header, _) = split_header(DRAFT).unwrap();
    let mut header: Value = serde_json::from_str(&header).unwrap();
    header[field] = value;
    format!("{HEADER_START}\n{header}\n{HEADER_END}\n\n{INTENT}\n\nRead the work order.\n\n{ACCEPTANCE}\n\n- Read it.\n")
}

fn reference(tracker: &str, link: &str) -> String {
    draft_with(
        "references",
        json!([{"tracker": tracker, "item": "TEAM-1", "link": link}]),
    )
}

#[test]
fn canonical_documents_round_trip() {
    let trackers = trackers();
    let assigned = WorkOrder::parse(ASSIGNED, &trackers).unwrap();
    assert_eq!(assigned.id(), Some(&"WO-12".parse().unwrap()));
    assert_eq!(assigned.title(), "Render the work order");
    assert_eq!(assigned.state(), WorkOrderState::Ruled);
    assert_eq!(assigned.intent(), "Keep the operator's rulings with the work.\n\nThe document gives reviews a stable statement of the work.");
    assert_eq!(
        assigned.acceptance(),
        [
            "Render an assigned work order byte for byte.",
            "Read a draft without an id.",
        ]
    );
    assert_eq!(
        assigned.references(),
        [Reference {
            tracker: "github".into(),
            item: "feedback-loop-ai/brokkr#1272".into(),
            link: "https://github.com/feedback-loop-ai/brokkr/issues/1272".into(),
        }]
    );
    let draft = WorkOrder::parse(DRAFT, &trackers).unwrap();
    assert_eq!(draft.id(), None);
    assert_eq!(draft.state(), WorkOrderState::Proposed);
    for (document, order) in [(ASSIGNED, assigned), (DRAFT, draft)] {
        assert_eq!(order.render(), document);
        assert_eq!(WorkOrder::parse(&order.render(), &trackers).unwrap(), order);
    }
}

#[test]
fn noncanonical_document_renders_in_contract_order() {
    let trackers = trackers();
    let order = WorkOrder::parse(NONCANONICAL, &trackers).unwrap();
    assert_eq!(order.render(), DRAFT);
    let crlf = NONCANONICAL.replace('\n', "\r\n");
    let reread = WorkOrder::parse(&crlf, &trackers).unwrap();
    assert_eq!(reread, order);
    assert_eq!(reread.digest(), text_digest(&crlf));
    assert_ne!(reread.digest(), order.digest());
}

#[test]
fn unknown_header_key_is_refused() {
    let message = "unknown field `extra`, expected one of `id`, `title`, `state`, `rulings`, `asks`, `constraints`, `caps`, `references`, `parent`, `edges` at line 1 column 131";
    assert_refused(
        UNKNOWN_KEY,
        &trackers(),
        Refusal::HeaderJson(message.into()),
        &format!("the work order's JSON header is invalid: {message}"),
    );
}

#[test]
fn unclassified_ending_is_refused() {
    assert_refused(
        UNCLASSIFIED,
        &trackers(),
        Refusal::Unclassified,
        "unclassified cannot be named as a work order edge condition",
    );
}

#[test]
fn off_host_reference_is_refused() {
    assert_refused(
        &OFF_HOST.replace("github.com@", ""),
        &trackers(),
        Refusal::ReferenceHost {
            tracker: "github".into(),
            host: "elsewhere.example".into(),
        },
        "host 'elsewhere.example' is outside the link hosts for tracker 'github'",
    );
}

#[test]
fn invalid_header_fixtures_have_named_refusals() {
    use Refusal::*;
    let trackers = trackers();
    let invalid_headers = [
        (
            fixture!("bad-id.md"),
            Id,
            "a work order id must be WO- followed by a positive 32-bit integer without leading zeroes",
        ),
        (
            fixture!("two-line-title.md"),
            Title,
            "the work order's title must occupy one non-empty line",
        ),
        (
            fixture!("bad-date.md"),
            RulingDate,
            "a work order ruling date must be a calendar date in YYYY-MM-DD form",
        ),
        (
            fixture!("bad-cite.md"),
            RulingCite,
            "a work order ruling citation must be an absolute HTTPS link",
        ),
        (
            fixture!("malformed-ruling-cite.md"),
            RulingCite,
            "a work order ruling citation must be an absolute HTTPS link",
        ),
        (
            fixture!("zero-ask.md"),
            AskNumber,
            "a work order ask number must be positive",
        ),
        (
            fixture!("bad-crate.md"),
            CapCrate,
            "each public-API cap must be keyed by a crate name",
        ),
        (
            fixture!("unknown-condition.md"),
            Condition,
            "a work order edge condition must name completed, ended, merged or an ending",
        ),
        (
            fixture!("http-link.md"),
            ReferenceLink,
            "a work order reference must carry an absolute HTTPS link",
        ),
        (
            fixture!("malformed-reference-link.md"),
            ReferenceLink,
            "a work order reference must carry an absolute HTTPS link",
        ),
    ];
    for (document, refusal, text) in invalid_headers {
        assert_refused(document, &trackers, refusal, text);
    }
    let row = crate::contract_lifecycle::row("contracts/work-order.v1.schema.json");
    assert_eq!(row.version, "work-order/v1");
    assert_eq!(row.reader, "load brokkr_core::work_order::Header::read");
    let (header, _) = split_header(ASSIGNED).unwrap();
    assert_eq!(Header::read(&header).unwrap().state, WorkOrderState::Ruled);
}

#[test]
fn invalid_body_and_context_fixtures_have_named_refusals() {
    use Refusal::*;
    let trackers = trackers();
    let invalid_documents = [
        (
            fixture!("before-header.md"),
            HeaderStart,
            "a work order must start with a fenced JSON header",
        ),
        (
            fixture!("unclosed-header.md"),
            HeaderEnd,
            "the work order's JSON header needs a closing fence",
        ),
        (
            fixture!("missing-ruling.md"),
            AskRuling,
            "a work order ask must name an existing ruling by its one-based position",
        ),
        (
            fixture!("unknown-tracker.md"),
            UnknownTracker("other".into()),
            "the work order cites unknown tracker 'other'",
        ),
        (
            fixture!("missing-intent.md"),
            MissingIntent,
            "the work order needs an '## Intent' section after its header",
        ),
        (
            fixture!("missing-acceptance.md"),
            MissingAcceptance,
            "the work order needs an '## Acceptance' section after its intent",
        ),
        (
            fixture!("repeated-intent.md"),
            RepeatedIntent,
            "the work order repeats its '## Intent' section",
        ),
        (
            fixture!("repeated-acceptance.md"),
            RepeatedAcceptance,
            "the work order repeats its '## Acceptance' section",
        ),
        (
            fixture!("other-section.md"),
            OtherSection,
            "a work order has only '## Intent' and '## Acceptance' sections",
        ),
        (
            fixture!("empty-intent.md"),
            EmptyIntent,
            "the work order's intent must contain prose",
        ),
        (
            fixture!("empty-acceptance.md"),
            EmptyAcceptance,
            "the work order's acceptance must contain at least one list item",
        ),
        (
            fixture!("bad-acceptance.md"),
            AcceptanceItem,
            "each work order acceptance item must be a non-empty list item on one line",
        ),
    ];
    for (document, refusal, text) in invalid_documents {
        assert_refused(document, &trackers, refusal, text);
    }
}

#[test]
fn ids_states_and_conditions_are_closed() {
    for id in ["WO-1", "WO-9", "WO-10", "WO-4294967295"] {
        assert_eq!(id.parse::<WorkOrderId>().unwrap().to_string(), id);
    }
    for text in [
        "1",
        "WO-",
        "WO-0",
        "WO-01",
        "WO-1x",
        "WO-1\n",
        "WO-١",
        "wo-1",
        "WO-+1",
        "WO-4294967296",
    ] {
        assert_eq!(
            text.parse::<WorkOrderId>().unwrap_err().0,
            Refusal::Id,
            "{text:?}"
        );
    }
    let trackers = trackers();
    for (word, state) in [
        ("proposed", WorkOrderState::Proposed),
        ("ruled", WorkOrderState::Ruled),
        ("queued", WorkOrderState::Queued),
        ("in-progress", WorkOrderState::InProgress),
        ("shipped", WorkOrderState::Shipped),
        ("merged", WorkOrderState::Merged),
        ("withdrawn", WorkOrderState::Withdrawn),
        ("superseded", WorkOrderState::Superseded),
        ("delivered-elsewhere", WorkOrderState::DeliveredElsewhere),
    ] {
        let draft = draft_with("state", json!(word));
        let order = WorkOrder::parse(&draft, &trackers).unwrap();
        assert_eq!(order.state(), state);
        assert_eq!(serde_json::to_value(order.state()).unwrap(), word);
    }

    let builtins = [
        ("completed", Condition::Completed),
        ("ended", Condition::Ended),
        ("merged", Condition::Merged),
    ];
    let endings = Ending::ALL
        .into_iter()
        .filter(|ending| *ending != Ending::Unclassified)
        .map(|ending| (ending.word(), Condition::Ending(ending)));
    for (word, condition) in builtins.into_iter().chain(endings) {
        let draft = draft_with("edges", json!([{"awaits": "WO-1", "condition": word}]));
        let order = WorkOrder::parse(&draft, &trackers).unwrap();
        assert_eq!(order.parsed.header.edges.0[0].condition, condition);
        assert_eq!(serde_json::to_value(condition).unwrap(), word);
    }
}

#[test]
fn header_shapes_are_checked() {
    let trackers = trackers();
    for (field, value) in [
        ("id", Value::Null),
        ("parent", Value::Null),
        ("state", json!("done")),
        (
            "rulings",
            json!([["Keep the text.", "2026-11-23", "https://example.com/1"]]),
        ),
        (
            "rulings",
            json!([{"words":"Keep the text.", "date":"2026-11-23", "cite":"https://example.com/1", "extra":1}]),
        ),
        (
            "asks",
            json!([{"number":1, "question":"Where?", "ruling":null}]),
        ),
        (
            "asks",
            json!([{"number":1, "question":"Where?", "ruling":1.5}]),
        ),
        ("asks", json!([{"number":1.5, "question":"Where?"}])),
        (
            "asks",
            json!([{"number":1, "question":"Where?", "extra":1}]),
        ),
        ("edges", json!([["WO-1", "merged"]])),
        (
            "edges",
            json!([{"awaits":"WO-1", "condition":"merged", "extra":1}]),
        ),
        (
            "references",
            json!([{"tracker":"github", "item":"1", "link":"https://github.com/1", "extra":1}]),
        ),
        ("caps", json!({"brokkr-core": 4294967296_u64})),
        ("caps", json!({"brokkr-core": -1})),
        ("caps", json!({"brokkr-core": 0.5})),
        ("caps", json!({"brokkr-core": "1"})),
    ] {
        let bad = draft_with(field, value);
        let error = WorkOrder::parse(&bad, &trackers).unwrap_err();
        assert!(matches!(error.0, Refusal::HeaderJson(_)), "{error}");
    }
    for (field, value, refusal) in [
        ("title", json!("  "), Refusal::Title),
        ("parent", json!("WO-0"), Refusal::Id),
        (
            "edges",
            json!([{"awaits":"1", "condition":"merged"}]),
            Refusal::Id,
        ),
        (
            "rulings",
            json!([{"words":"Keep the text.", "date":"2026-2-03", "cite":"https://example.com/1"}]),
            Refusal::RulingDate,
        ),
        (
            "rulings",
            json!([{"words":"Keep the text.", "date":"2026-W48-1", "cite":"https://example.com/1"}]),
            Refusal::RulingDate,
        ),
        (
            "asks",
            json!([{"number":1, "question":"Where?", "ruling":0}]),
            Refusal::AskRuling,
        ),
        ("caps", json!({"": 15}), Refusal::CapCrate),
    ] {
        let bad = draft_with(field, value);
        assert_eq!(WorkOrder::parse(&bad, &trackers).unwrap_err().0, refusal);
    }
    let cap = draft_with("caps", json!({"x2_max-limit": u32::MAX as f64, "zero":0.0}));
    assert_eq!(
        WorkOrder::parse(&cap, &trackers)
            .unwrap()
            .parsed
            .header
            .caps,
        BTreeMap::from([
            ("x2_max-limit".into(), Count(u32::MAX)),
            ("zero".into(), Count(0))
        ]),
    );
}

#[test]
fn body_boundaries_are_checked() {
    let trackers = trackers();
    for (source, replacement, refusal) in [
        (INTENT, "", Refusal::MissingIntent),
        (INTENT, "## Other", Refusal::OtherSection),
        ("Keep the work clear.", "##\tOther", Refusal::OtherSection),
        ("Keep the work clear.", "##", Refusal::OtherSection),
        ("- Read the work order.", INTENT, Refusal::RepeatedIntent),
        ("- Read the work order.", "-", Refusal::AcceptanceItem),
        (
            "- Read the work order.",
            ". Read it.",
            Refusal::AcceptanceItem,
        ),
        (
            "- Read the work order.",
            "1234567890. Text.",
            Refusal::AcceptanceItem,
        ),
        (
            "- Read the work order.",
            "x. Text.",
            Refusal::AcceptanceItem,
        ),
        ("- Read the work order.", "-   ", Refusal::AcceptanceItem),
    ] {
        let bad = DRAFT.replace(source, replacement);
        assert_eq!(
            WorkOrder::parse(&bad, &trackers).unwrap_err().0,
            refusal,
            "{bad}"
        );
    }
    let text = DRAFT
        .replace(
            "Keep the work clear.",
            "Keep the work clear.\n\n### Note\n\nUse one document.",
        )
        .replace(
            "- Read the work order.",
            "+ Read the work order.\n\n*\tRead the header.\n1. Read the intent.\n2) Read the output.",
        );
    let order = WorkOrder::parse(&text, &trackers).unwrap();
    assert_eq!(
        order.intent(),
        "Keep the work clear.\n\n### Note\n\nUse one document."
    );
    assert_eq!(
        order.acceptance(),
        [
            "Read the work order.",
            "Read the header.",
            "Read the intent.",
            "Read the output."
        ]
    );
}

#[test]
fn intent_line_endings_are_stable_after_rendering() {
    let trackers = trackers();
    let canonical = DRAFT.replace("Keep the work clear.", "First line.\nSecond line.");
    let source = canonical.replace('\n', "\r\n");
    let order = WorkOrder::parse(&source, &trackers).unwrap();
    assert_eq!(order.intent(), "First line.\nSecond line.");
    assert_eq!(order.render(), canonical);
    assert_eq!(WorkOrder::parse(&canonical, &trackers).unwrap(), order);
}

#[test]
fn lone_carriage_returns_in_the_body_are_refused() {
    let trackers = trackers();
    assert_refused(
        fixture!("lone-carriage-return.md"),
        &trackers,
        Refusal::BodyCarriageReturn,
        "the work order's Markdown body must use LF or CRLF line endings",
    );
    for document in [
        DRAFT.replace(
            "- Read the work order.",
            "- Read the work order.\r## Other\r\rExtra prose.",
        ),
        DRAFT.replace("Keep the work clear.", "First line.\r\r\nSecond line."),
        format!("{}\r", DRAFT.trim_end_matches('\n')),
    ] {
        assert_eq!(
            WorkOrder::parse(&document, &trackers).unwrap_err().0,
            Refusal::BodyCarriageReturn
        );
    }
}

#[test]
fn https_links_preserve_valid_spelling_and_refuse_repairs() {
    for (link, host) in [
        ("HTTPS://work.example", "work.example"),
        ("https://Work.example:/", "Work.example"),
        (
            "https://work.example:00065535/a@b?a=b@c&c=%ab#x@y",
            "work.example",
        ),
        ("https://[2001:db8::1]:1/project", "[2001:db8::1]"),
        ("https://%77ork.example/project", "%77ork.example"),
    ] {
        assert_eq!(https_uri_host(link), Some(host), "{link}");
    }
    for link in [
        "http://work.example/",
        "https:work.example/",
        "https:///work.example/",
        "https://name@:42/",
        "https://guest:p%61ss@work.example/",
        "https://one@two@work.example/",
        "https://g%1z@work.example/",
        "https://work.example:65536/",
        "https://work.example:+1/",
        "https://[::1",
        "https://[broken]/",
        "https://[::1]extra/",
        "https://work.example/a%2",
        "https://work.example/a%z0",
        "https://work.example/a%0z",
        "https://work.example/a\"b",
        "https://work.example/x#y#z",
        "https://work.example/x\u{e9}",
    ] {
        assert_eq!(https_uri_host(link), None, "{link}");
    }

    let trackers = trackers();
    let link = reference("github", "https://%FF/issue/1");
    assert_eq!(
        WorkOrder::parse(&link, &trackers).unwrap_err().0,
        Refusal::ReferenceHost {
            tracker: "github".into(),
            host: "%FF".into(),
        }
    );
}

#[test]
fn tracker_data_controls_reference_hosts() {
    for (word, definition, link) in DEFINITIONS {
        let one = Trackers::from_definitions([definition]).unwrap();
        let order = WorkOrder::parse(&reference(word, link), &one).unwrap();
        assert_eq!(
            order.references(),
            [Reference {
                tracker: word.into(),
                item: "TEAM-1".into(),
                link: link.into(),
            }]
        );
        let off = reference(word, "https://elsewhere.example/T-1");
        assert_eq!(
            WorkOrder::parse(&off, &one).unwrap_err().0,
            Refusal::ReferenceHost {
                tracker: word.into(),
                host: "elsewhere.example".into()
            },
        );
    }
    let trackers = Trackers::from_definitions([
        r#"{"tracker":"local2-tracker","hosts":["WORK.example","*.adopter.example"]}"#,
    ])
    .unwrap();
    for link in [
        "HTTPS://WORK.EXAMPLE/project/1",
        "https://tenant.adopter.example/project/1",
        "https://tenant.sub.adopter.example:8443/project/1",
    ] {
        let document = reference("local2-tracker", link);
        assert_eq!(
            WorkOrder::parse(&document, &trackers).unwrap().references()[0].link,
            link
        );
    }
    for host in [
        "adopter.example",
        ".adopter.example",
        "notadopter.example",
        "bad..adopter.example",
        "work.example.other",
    ] {
        let off = reference("local2-tracker", &format!("https://{host}/project/1"));
        assert_eq!(
            WorkOrder::parse(&off, &trackers).unwrap_err().0,
            Refusal::ReferenceHost {
                tracker: "local2-tracker".into(),
                host: host.into()
            },
        );
    }
    for link in [
        "relative/item",
        "https:///work.example/1",
        "https://?work.example",
        "https://#work.example",
        "https://",
        "https://work.example:invalid",
        "https://work.example/a b",
        "https://work.example/a\tb",
        "https://work.example\\path",
    ] {
        let bad = reference("local2-tracker", link);
        assert_eq!(
            WorkOrder::parse(&bad, &trackers).unwrap_err().0,
            Refusal::ReferenceLink,
            "{link}"
        );
    }
}

#[test]
fn invalid_tracker_definitions_are_refused() {
    use Refusal::*;
    let malformed = "invalid type: sequence, expected a JSON object at line 1 column 0";
    for (definition, refusal, text) in [
        ("[]", TrackerJson(malformed.into()), format!("the tracker definition is invalid: {malformed}")),
        (r#"{"tracker":"Local","hosts":["host.example"]}"#, TrackerWord, "a tracker word must start with a lowercase letter and use lowercase letters, digits or hyphens".into()),
        (r#"{"tracker":"local!","hosts":["host.example"]}"#, TrackerWord, "a tracker word must start with a lowercase letter and use lowercase letters, digits or hyphens".into()),
        (r#"{"tracker":"local","hosts":[]}"#, TrackerHosts, "a tracker definition must list at least one link host".into()),
        (r#"{"tracker":"local","hosts":["*"]}"#, TrackerHostPattern, "a tracker link host pattern must be a DNS name or a DNS suffix starting with '*.'".into()),
        (r#"{"tracker":"local","hosts":["-bad.example"]}"#, TrackerHostPattern, "a tracker link host pattern must be a DNS name or a DNS suffix starting with '*.'".into()),
        (r#"{"tracker":"local","hosts":["bad-.example"]}"#, TrackerHostPattern, "a tracker link host pattern must be a DNS name or a DNS suffix starting with '*.'".into()),
    ] {
        let error = Trackers::from_definitions([definition]).unwrap_err();
        assert_eq!(error.0, refusal);
        assert_eq!(error.to_string(), text);
    }
    let definition = tracker!("github");
    let error = Trackers::from_definitions([definition, definition]).unwrap_err();
    assert_eq!(error.0, DuplicateTracker);
    assert_eq!(
        error.to_string(),
        "each tracker word must have exactly one definition"
    );
}

#[test]
fn work_order_id_exposes_its_positive_number() {
    let number = u32::MAX;
    let id = WorkOrderId::from_number(number).unwrap();
    assert_eq!(id, "WO-4294967295".parse().unwrap());
    assert_eq!(id.number(), number);
    assert_eq!(WorkOrderId::from_number(0).unwrap_err().0, Refusal::Id);
}

#[test]
fn with_id_changes_only_the_id() {
    let order = WorkOrder::parse(ASSIGNED, &trackers()).unwrap();
    let expected = ASSIGNED.replace(r#""id": "WO-12""#, r#""id": "WO-42""#);
    assert_eq!(order.with_id("WO-42".parse().unwrap()).render(), expected);
}

#[test]
fn with_state_changes_only_the_state() {
    let order = WorkOrder::parse(ASSIGNED, &trackers()).unwrap();
    let expected = ASSIGNED.replace(r#""state": "ruled""#, r#""state": "proposed""#);
    assert_eq!(
        order.with_state(WorkOrderState::Proposed).render(),
        expected
    );
}

#[test]
fn open_asks_counts_every_ask_without_a_ruling() {
    let trackers = trackers();
    let closed = WorkOrder::parse(ASSIGNED, &trackers).unwrap();
    assert_eq!(closed.open_asks(), 0);
    let text = ASSIGNED.replace(",\n      \"ruling\": 1", "").replace(
        "\"question\": \"What assigns the id?\"",
        "\"question\": \"What assigns the id?\",\n      \"ruling\": 1",
    );
    let mixed = WorkOrder::parse(&text, &trackers).unwrap();
    assert_eq!(mixed.state(), WorkOrderState::Ruled);
    assert_eq!(mixed.open_asks(), 1);
    let text = text.replace(",\n      \"ruling\": 1", "");
    assert_eq!(WorkOrder::parse(&text, &trackers).unwrap().open_asks(), 2);
}

#[test]
fn journaled_open_asks_reads_the_document_without_tracker_definitions() {
    let text = ASSIGNED.replacen(",\n      \"ruling\": 1", "", 1);
    assert_eq!(WorkOrder::journaled_open_asks(&text).unwrap(), 1);
    assert_eq!(
        WorkOrder::journaled_open_asks("unreadable").unwrap_err().0,
        Refusal::HeaderStart
    );
}
