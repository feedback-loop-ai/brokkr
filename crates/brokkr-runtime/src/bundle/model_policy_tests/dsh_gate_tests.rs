//! dsh judges with `qwen4-max` alone (decision 0021 addendum 2026-11-22):
//! the shipped adapter's trusted tier lets a gate stand on dsh, and its
//! `judges` list (decision 0041 ruling 3) confines that gate to the one
//! model the wager, the lane trial and the escalation replay measured.

use super::*;

/// A gate seat on the dsh driver that pins `model` at `high`, the level
/// the wager and the lane trial seated Qwen 4.0 Max at.
fn dsh_gate(model: &str) -> Value {
    let mut work = seat("dsh", Some("gate"), None);
    work["driver"]["command"] =
        json!(["{brokkr}", "driver", "dsh", "--", "--model", model, "--effort", "high"]);
    work
}

/// The shipped `adapters/dsh.json`, parsed, for a test to edit one field.
fn shipped_dsh() -> Value {
    serde_json::from_slice(&std::fs::read(shipped_adapters().join("dsh.json")).unwrap()).unwrap()
}

/// The refusal text a compile produced, in its typed variant.
fn invalid(compiled: Result<Bundle, CompileError>) -> String {
    match compiled {
        Err(CompileError::Invalid(text)) => text,
        other => panic!(
            "expected CompileError::Invalid, got {:?}",
            other.map(|_| ())
        ),
    }
}

#[test]
fn a_gate_on_the_shipped_dsh_adapter_pinned_to_qwen4_max_compiles() {
    let fixture = Fixture::new();
    fixture
        .compile_against(dsh_gate("dashscope/qwen4-max"), &shipped_adapters())
        .expect("the trusted dsh adapter declares qwen4-max in 'judges'");
}

#[test]
fn a_gate_on_the_shipped_dsh_adapter_pinned_to_any_other_model_is_refused() {
    let fixture = Fixture::new();
    let dsh = shipped_dsh();
    let others = dsh["models"].as_object().unwrap().iter();
    let others: Vec<_> = others.filter(|(alias, _)| *alias != "qwen4-max").collect();
    assert_eq!(others.len(), 16, "every other model the shipped dsh maps");
    for (alias, concrete) in others {
        let compiled =
            fixture.compile_against(dsh_gate(concrete.as_str().unwrap()), &shipped_adapters());
        assert_eq!(
            invalid(compiled),
            format!(
                "seat 'work' gate link 1 names model '{alias}', which driver 'dsh' does not \
                 declare in 'judges' (decision 0041 ruling 3 — an absent declaration is empty)"
            ),
        );
    }
}

/// The removal controls: the first test's gate, against the shipped dsh
/// with `judges` emptied, and with the tier set back to untrusted, is
/// refused, so neither half of the grant is idle.
#[test]
fn the_qwen4_max_gate_is_refused_once_either_half_of_the_grant_is_removed() {
    let judges_emptied = {
        let mut dsh = shipped_dsh();
        dsh["judges"] = json!([]);
        dsh
    };
    let untrusted = {
        let mut dsh = shipped_dsh();
        dsh["trust_tier"] = json!("untrusted");
        dsh
    };
    for (dsh, refusal) in [
        (
            judges_emptied,
            "seat 'work' gate link 1 names model 'qwen4-max', which driver 'dsh' does not \
             declare in 'judges' (decision 0041 ruling 3 — an absent declaration is empty)",
        ),
        (
            untrusted,
            "seat 'work' is gate class but seats driver 'dsh', which does not hold the \
             trusted tier; a gate seat IS the check, and nobody stands behind the judges \
             (decision 0021 ruling 2 — an undeclared tier is untrusted)",
        ),
    ] {
        let fixture = Fixture::new();
        fixture.write_adapter(dsh);
        let compiled = fixture.compile(dsh_gate("dashscope/qwen4-max"));
        assert_eq!(invalid(compiled), refusal);
    }
}
