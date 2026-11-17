//! Decision 0110 ruling 7: `forge.host/v4`'s required run ceiling, read
//! by the loader as the published contract admits it, and every earlier
//! version still read with none.

use serde_json::{json, Value};

use super::super::tests::contract_validator;
use super::super::{config, read, HostError, Problem as HostProblem};
use super::*;

/// A v4 host declaring a run ceiling of two, and nothing it need not.
fn v4() -> Value {
    json!({"schema": "forge.host/v4", "providers": {},
           "floors": [{"place": "temp", "floor_bytes": 1}],
           "boxed_builds": {"ceiling": 1}, "runs": {"ceiling": 2}})
}

/// `host` with `runs` set to `runs`, or left out where it is `None`, and
/// written as `schema`.
fn with(schema: &str, runs: Option<Value>) -> Value {
    let mut host = v4();
    host["schema"] = json!(schema);
    let map = host.as_object_mut().unwrap();
    match runs {
        Some(runs) => map.insert("runs".into(), runs),
        None => map.remove("runs"),
    };
    host
}

/// The problem the loader refuses `host` with.
fn refused(host: &Value) -> HostProblem {
    match config(host.to_string().as_bytes()) {
        Err(problem) => problem,
        Ok(read) => panic!("the loader admitted {host}: {read:?}"),
    }
}

/// The published v4 contract, judged by its own JSON Schema.
fn contract() -> jsonschema::Validator {
    contract_validator("host.v4.schema.json")
}

#[test]
fn a_v4_ceiling_reads_and_every_earlier_version_reads_with_none() {
    let read = config(v4().to_string().as_bytes()).unwrap();
    assert_eq!(read.runs, NonZeroU32::new(2));
    assert_eq!(read.schema, Schema::V4);
    for (schema, version) in [
        ("forge.host/v1", Schema::V1),
        ("forge.host/v2", Schema::V2),
        ("forge.host/v3", Schema::V3),
    ] {
        let read = config(with(schema, None).to_string().as_bytes()).unwrap();
        assert_eq!((read.runs, read.schema), (None, version), "{schema}");
    }
}

/// The guide's sample host is a v4 file the published contract and the
/// loader both admit, declaring a run ceiling of three (6d L1).
#[test]
fn the_guide_sample_is_a_v4_file_the_contract_and_the_loader_admit() {
    let sample = super::super::tests::guide_sample();
    assert!(contract().is_valid(&sample), "{sample}");
    let read = config(sample.to_string().as_bytes()).unwrap();
    assert_eq!((read.schema, read.runs), (Schema::V4, NonZeroU32::new(3)));
}

#[test]
fn a_v4_file_without_runs_refuses_naming_the_v4_contract() {
    let problem = refused(&with("forge.host/v4", None));
    assert!(
        matches!(problem, HostProblem::Runs(Problem::Missing)),
        "{problem:?}"
    );
    assert_eq!(
        problem.to_string(),
        "forge.host/v4 declares runs.ceiling, and the file writes no runs"
    );
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("host.json");
    std::fs::write(&file, with("forge.host/v4", None).to_string()).unwrap();
    let Err(invalid @ HostError::Invalid { .. }) = read(&file) else {
        panic!("a v4 file without runs was read");
    };
    assert_eq!(
        invalid.to_string(),
        format!(
            "the host configuration at {} is not a valid forge.host/v4 file",
            file.display()
        )
    );
}

#[test]
fn a_run_ceiling_below_one_or_with_another_key_is_a_shape_refusal() {
    for (runs, said) in [
        (
            json!({"ceiling": 0}),
            "invalid value: integer `0`, expected a nonzero u32",
        ),
        (
            json!({"ceiling": -1}),
            "invalid value: integer `-1`, expected a nonzero u32",
        ),
        (
            json!({"ceiling": 2, "drivers": 1}),
            "unknown field `drivers`, expected `ceiling`",
        ),
        (json!(null), "invalid type: null, expected a JSON object"),
    ] {
        let host = with("forge.host/v4", Some(runs.clone()));
        let HostProblem::Shape(error) = refused(&host) else {
            panic!("{runs} was not a shape refusal");
        };
        let text = error.to_string();
        assert!(text.starts_with(said), "{runs}: {text}");
        assert!(!contract().is_valid(&host), "the schema admitted {runs}");
    }
    assert!(contract().is_valid(&v4()), "the schema refused a v4 host");
    assert!(
        !contract().is_valid(&with("forge.host/v4", None)),
        "the schema admitted a v4 host without runs"
    );
}

#[test]
fn runs_in_an_earlier_version_is_a_v4_only_key() {
    for schema in ["forge.host/v1", "forge.host/v2", "forge.host/v3"] {
        let problem = refused(&with(schema, Some(json!({"ceiling": 2}))));
        assert!(
            matches!(problem, HostProblem::Runs(Problem::NotV4(declared)) if declared == schema),
            "{problem:?}"
        );
        assert_eq!(
            problem.to_string(),
            format!("{schema} declares no runs; only forge.host/v4 does")
        );
        assert_eq!(problem.contract(), schema);
    }
    assert_eq!(
        HostProblem::Runs(Problem::Missing).contract(),
        "forge.host/v4"
    );
}

#[test]
fn doctor_reads_the_version_of_a_host_file_that_declares_no_run_ceiling() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("host.json");
    assert_eq!(runs_undeclared(&file), None, "an absent host file");
    for (host, said) in [
        (with("forge.host/v3", None), Some("forge.host/v3")),
        (with("forge.host/v1", None), Some("forge.host/v1")),
        (v4(), None),
        (with("forge.host/v4", None), None),
    ] {
        std::fs::write(&file, host.to_string()).unwrap();
        assert_eq!(runs_undeclared(&file), said, "{host}");
    }
    assert_eq!(runs_undeclared(dir.path()), None, "an unreadable host file");
}

/// Decision 0110 ruling 7's `forge.host/v4` row names this crate's private
/// reader.
#[test]
fn the_lifecycle_row_names_the_reader_of_v4() {
    let row = crate::contract_lifecycle::row("contracts/host.v4.schema.json");
    assert_eq!(row.version, "forge.host/v4");
    assert_eq!(row.reader, "load brokkr_runtime::admission::host::read");
}
