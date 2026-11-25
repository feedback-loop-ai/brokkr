//! Decision 0125 ruling 7: `forge.host/v5`, read by the loader as its
//! published contract admits it, and every earlier version read with the
//! demand defaults and its boxed-build ceiling still required.

use std::num::{NonZeroU32, NonZeroU64};

use serde_json::{json, Value};

use super::super::tests::contract_validator;
use super::super::{config, Problem as HostProblem};
use super::*;

/// A v5 host with nothing it need not write: no run or boxed-build
/// ceiling, no demand block.
fn v5() -> Value {
    json!({"schema": "forge.host/v5", "providers": {},
           "floors": [{"place": "temp", "floor_bytes": 1}]})
}

/// `host` with `pointer`'s key set to `value`.
fn with(mut host: Value, key: &str, value: Value) -> Value {
    host[key] = value;
    host
}

fn refused(host: &Value) -> HostProblem {
    match config(host.to_string().as_bytes()) {
        Err(problem) => problem,
        Ok(read) => panic!("the loader admitted {host}: {read:?}"),
    }
}

/// The demand problem `host` is refused with.
fn demand_refusal(host: &Value) -> String {
    match refused(host) {
        HostProblem::Demand(problem) => problem.to_string(),
        other => panic!("{host} refused as {other:?}"),
    }
}

#[test]
fn a_bare_v5_reads_with_no_ceilings_and_the_demand_defaults() {
    let read = config(v5().to_string().as_bytes()).unwrap();
    assert_eq!((read.runs, read.boxed_builds), (None, None));
    assert_eq!(
        read.demand,
        Demand {
            window_runs: NonZeroU32::new(20).unwrap(),
            margin_percent: 25,
            recipes: BTreeMap::new(),
        }
    );
    assert!(contract_validator("host.v5.schema.json").is_valid(&v5()));
}

#[test]
fn a_v5_demand_block_reads_its_window_margin_and_declared_recipes() {
    let host = with(
        with(v5(), "runs", json!({"ceiling": 3})),
        "demand",
        json!({"window_runs": 5, "margin_percent": 0,
               "recipes": {"self": {"peak_memory_bytes": 4096, "peak_load_millicores": 1500}}}),
    );
    assert!(contract_validator("host.v5.schema.json").is_valid(&host));
    let read = config(host.to_string().as_bytes()).unwrap();
    assert_eq!(read.runs, NonZeroU32::new(3));
    let declared = Declared {
        peak_memory_bytes: NonZeroU64::new(4096).unwrap(),
        peak_load_millicores: NonZeroU64::new(1500).unwrap(),
    };
    assert_eq!(
        read.demand,
        Demand {
            window_runs: NonZeroU32::new(5).unwrap(),
            margin_percent: 0,
            recipes: BTreeMap::from([("self".to_string(), declared)]),
        }
    );
}

/// An earlier version keeps its boxed-build ceiling required and may write
/// no demand; v5's numbers out of range refuse in the loader's words, and
/// the published contract refuses each too.
#[test]
fn the_loader_and_the_v5_contract_refuse_what_the_version_may_not_write() {
    let mut v4 = with(v5(), "schema", json!("forge.host/v4"));
    v4["runs"] = json!({"ceiling": 1});
    assert_eq!(
        demand_refusal(&v4),
        "forge.host/v4 requires boxed_builds; only forge.host/v5 may leave it out"
    );
    v4["boxed_builds"] = json!({"ceiling": 1});
    assert_eq!(
        config(v4.to_string().as_bytes())
            .unwrap()
            .demand
            .margin_percent,
        25
    );
    assert_eq!(
        demand_refusal(&with(v4, "demand", json!({}))),
        "forge.host/v4 declares no demand; only forge.host/v5 does"
    );
    let validator = contract_validator("host.v5.schema.json");
    for (demand, says) in [
        (
            json!({"window_runs": 0}),
            "demand.window_runs is 0; it is a whole number from 1 to 1000",
        ),
        (
            json!({"window_runs": 1001}),
            "demand.window_runs is 1001; it is a whole number from 1 to 1000",
        ),
        (
            json!({"margin_percent": 1001}),
            "demand.margin_percent is 1001; it is a whole number from 0 to 1000",
        ),
    ] {
        let host = with(v5(), "demand", demand);
        assert_eq!(demand_refusal(&host), says);
        assert!(!validator.is_valid(&host), "{host}");
    }
    for demand in [
        json!({"window": 3}),
        json!({"recipes": {"self": {"peak_memory_bytes": 0, "peak_load_millicores": 1}}}),
        json!({"recipes": {"self": {"peak_memory_bytes": 1}}}),
    ] {
        let host = with(v5(), "demand", demand);
        assert!(matches!(refused(&host), HostProblem::Shape(_)), "{host}");
        assert!(!validator.is_valid(&host), "{host}");
    }
}
