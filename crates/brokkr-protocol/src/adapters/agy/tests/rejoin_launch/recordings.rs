//! Measured `l2c-*` stdout recordings (`docs/evidence/adapters/agy-cli-1-5/`)
//! folded through [`AgyFold`], checking launch terminal, typed final verdict
//! (or unsettled launch), and exact `session_meta` counters for every stream
//! (#1227).

use super::*;
use crate::adapters::agy::fold::Verdict;

const NEON_SLUG: &str = "gemini-4-neon";
const UUID_ISO: &str = "faa7e769-0ab7-4a47-b134-1282c2144460";
const UUID_BOX: &str = "eb33b726-8955-4cfd-bd92-80d433deb774";
const UUID_NEON: &str = "d78af25d-8347-44fe-b69c-113e1731c2e9";
const UUID_PERM: &str = "0c4e0aed-f160-4a2c-8bcd-988719249840";

#[derive(Debug, PartialEq)]
enum FoldOutcome {
    Settled(Verdict),
    Unsettled,
}

#[derive(Clone, Copy)]
enum ExpectedOutcome {
    Answer(&'static str),
    Unsettled,
}

#[derive(Clone, Copy)]
struct UsageExpect {
    input: u64,
    output: u64,
    reasoning: Option<u64>,
    cache: Option<u64>,
    turns: Option<u64>,
}

struct RecordedCase {
    file: &'static str,
    pin: &'static str,
    offered: Option<&'static str>,
    terminal: LaunchTerminal,
    outcome: ExpectedOutcome,
    usage: Option<UsageExpect>,
}

const fn settled_case(
    file: &'static str,
    pin: &'static str,
    offered: Option<&'static str>,
    answer: &'static str,
    usage: UsageExpect,
) -> RecordedCase {
    let terminal = match offered {
        Some(_) => LaunchTerminal::Resumed,
        None => LaunchTerminal::Cold,
    };
    RecordedCase {
        file,
        pin,
        offered,
        terminal,
        outcome: ExpectedOutcome::Answer(answer),
        usage: Some(usage),
    }
}

const fn mismatch_case(file: &'static str, offered: &'static str) -> RecordedCase {
    RecordedCase {
        file,
        pin: FLASH,
        offered: Some(offered),
        terminal: LaunchTerminal::Mismatch,
        outcome: ExpectedOutcome::Unsettled,
        usage: None,
    }
}

const fn usage(
    input: u64,
    output: u64,
    reasoning: Option<u64>,
    cache: Option<u64>,
    turns: Option<u64>,
) -> UsageExpect {
    UsageExpect {
        input,
        output,
        reasoning,
        cache,
        turns,
    }
}

const RECORDED_L2C: &[RecordedCase] = &[
    settled_case(
        "l2c-01-cold.ndjson",
        FLASH,
        None,
        "STORED\n",
        usage(9_575, 2, None, None, Some(1)),
    ),
    settled_case(
        "l2c-02-rejoin-kept.ndjson",
        FLASH,
        Some(UUID_A),
        "The code word was **PLUM-7731**. \n\nI have remembered the second code word: **FIG-4410**.\n",
        usage(9_801, 31, None, None, None),
    ),
    settled_case(
        "l2c-03-rejoin-chain.ndjson",
        FLASH,
        Some(UUID_A),
        "PLUM-7731 FIG-4410\n",
        usage(10_049, 13, None, None, None),
    ),
    mismatch_case("l2c-04-unknown-id.ndjson", UUID_OTHER),
    mismatch_case("l2c-05-malformed-id.ndjson", "not-an-id"),
    settled_case(
        "l2c-06-db-only.ndjson",
        FLASH,
        Some(UUID_A),
        "PLUM-7731\n",
        usage(9_793, 7, None, None, None),
    ),
    mismatch_case("l2c-07-brain-only.ndjson", UUID_A),
    settled_case(
        "l2c-08-rejoin-other-model.ndjson",
        NEON_SLUG,
        Some(UUID_A),
        "PLUM-7731\n",
        usage(9_880, 67, Some(59), None, None),
    ),
    settled_case(
        "l2c-09-rejoin-other-cwd.ndjson",
        FLASH,
        Some(UUID_A),
        "PLUM-7731\n",
        usage(9_806, 7, None, None, None),
    ),
    settled_case(
        "l2c-10-iso-cold.ndjson",
        FLASH,
        None,
        "Created At: 2026-11-27T06:07:22+02:00\nCompleted At: 2026-11-27T06:07:22+02:00\nSENTINEL-NONCE-COLD\n",
        usage(16_543 + 24_280, 236, None, Some(24_280), Some(1)),
    ),
    settled_case(
        "l2c-11-iso-rejoin.ndjson",
        FLASH,
        Some(UUID_ISO),
        "NO SENTINEL TOOL\nPEAR-5527\n",
        usage(12_870 + 8_724, 49, None, Some(8_724), None),
    ),
    settled_case(
        "l2c-12-box-cold.ndjson",
        FLASH,
        None,
        "KIWI-seed\n",
        usage(14_392 + 15_963, 96, None, Some(15_963), Some(1)),
    ),
    settled_case(
        "l2c-13-box-rejoin.ndjson",
        FLASH,
        Some(UUID_BOX),
        "KIWI-2093 again\n",
        usage(12_722 + 8_530, 55, None, Some(8_530), None),
    ),
    settled_case(
        "l2c-14-par-alpha.ndjson",
        FLASH,
        Some(UUID_A),
        "ALPHA PLUM-7731\n",
        usage(9_806, 8, None, None, None),
    ),
    settled_case(
        "l2c-15-par-beta.ndjson",
        FLASH,
        Some(UUID_A),
        "ALPHA PLUM-7731\nBETA PLUM-7731\n",
        usage(10_016, 18, None, None, None),
    ),
    settled_case(
        "l2c-16-par-after.ndjson",
        FLASH,
        Some(UUID_A),
        "Remember this code\nReply with exactly:\nList, one per\n",
        usage(2_428 + 7_763, 13, None, Some(7_763), None),
    ),
    settled_case(
        "l2c-17-neon-cold.ndjson",
        NEON_SLUG,
        None,
        "READY\n",
        usage(9_582, 39, Some(37), None, Some(1)),
    ),
    settled_case(
        "l2c-18-neon-rejoin.ndjson",
        NEON_SLUG,
        Some(UUID_NEON),
        "LIME-8802\n",
        usage(9_912, 37, Some(29), None, None),
    ),
    settled_case(
        "l2c-19-perm-cold.ndjson",
        FLASH,
        None,
        "DONE\n",
        usage(11_762 + 7_548, 44, None, Some(7_548), Some(1)),
    ),
    settled_case(
        "l2c-20-perm-rejoin-gated.ndjson",
        FLASH,
        Some(UUID_PERM),
        "MANGO-6157\nREFUSED\n",
        usage(12_087 + 7_931, 55, None, Some(7_931), None),
    ),
    settled_case(
        "l2c-21-perm-rejoin-norule.ndjson",
        FLASH,
        Some(UUID_PERM),
        "MANGO-6157\nREFUSED\n",
        usage(12_096 + 7_931, 55, None, Some(7_931), None),
    ),
    settled_case(
        "l2c-22-perm-rejoin-allowed.ndjson",
        FLASH,
        Some(UUID_PERM),
        "MANGO-6157\nRAN\n",
        usage(12_127 + 7_931, 52, None, Some(7_931), None),
    ),
];

fn fold_recording(
    name: &str,
    pin: &str,
    offered: Option<&str>,
) -> (Map<String, Value>, LaunchTerminal, FoldOutcome) {
    let text = recorded_box(name);
    let mut fold = AgyFold::new(pin, &[]);
    let mut plan = LaunchPlan::cold(Vec::new(), "agy-conversation", None);
    if let Some(id) = offered {
        plan.rejoining = Some(id.to_string());
        plan.harness_version = Some("1.5.18".to_string());
    }
    let mut hold = LaunchHold::new("agy", plan);
    let mut record = Map::new();
    for line in text.lines() {
        match fold.read(line.as_bytes(), &mut record, &mut hold, &mut |_| {}) {
            Line::Read => {}
            Line::Stopped(refused) => {
                assert_eq!(fold.verdict(0, false), Verdict::Refused(refused.clone()));
                return (
                    record,
                    hold.terminal(),
                    FoldOutcome::Settled(Verdict::Refused(refused)),
                );
            }
            Line::Unsettled => return (record, hold.terminal(), FoldOutcome::Unsettled),
        }
    }
    (
        record,
        hold.terminal(),
        FoldOutcome::Settled(fold.verdict(0, false)),
    )
}

fn expected_record(case: &RecordedCase) -> Map<String, Value> {
    let mut map = Map::new();
    let Some(u) = case.usage else {
        return map;
    };
    map.insert("model".into(), json!(case.pin));
    map.insert("input_tokens".into(), json!(u.input));
    map.insert("output_tokens".into(), json!(u.output));
    if let Some(r) = u.reasoning {
        map.insert("reasoning_output_tokens".into(), json!(r));
    }
    if let Some(c) = u.cache {
        map.insert("cache_read_tokens".into(), json!(c));
    }
    if let Some(t) = u.turns {
        map.insert("num_turns".into(), json!(t));
    }
    map
}

#[test]
fn l6_rejoin_accounting_sums_step_rows_only_and_cold_matches_result() {
    let mut on_disk: Vec<String> = std::fs::read_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/evidence/adapters/agy-cli-1-5"),
    )
    .unwrap()
    .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
    .filter(|n| {
        n.starts_with("l2c-") && n.ends_with(".ndjson") && !n.ends_with(".transcript.ndjson")
    })
    .collect();
    on_disk.sort();
    let covered: Vec<&str> = RECORDED_L2C.iter().map(|c| c.file).collect();
    assert_eq!(
        on_disk, covered,
        "every l2c stdout recording must be in RECORDED_L2C"
    );

    for case in RECORDED_L2C {
        let (rec, term, outcome) = fold_recording(case.file, case.pin, case.offered);
        let expected_outcome = match case.outcome {
            ExpectedOutcome::Answer(text) => FoldOutcome::Settled(Verdict::Answer(text.into())),
            ExpectedOutcome::Unsettled => FoldOutcome::Unsettled,
        };
        assert_eq!(term, case.terminal, "{}", case.file);
        assert_eq!(outcome, expected_outcome, "{}", case.file);
        assert_eq!(rec, expected_record(case), "{}", case.file);
    }
}
