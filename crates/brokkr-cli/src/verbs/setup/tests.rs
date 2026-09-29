use super::*;
use brokkr_core::policy::audit::AuditError;

fn self_machine() -> Machine {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../bundles/self/policy.json");
    let table = std::fs::read_to_string(path).unwrap();
    Machine::from_table(&serde_json::from_str(&table).unwrap()).unwrap()
}

/// `brokkr compile` prints the core's audit, and a table over the budget
/// is diagnosed, not swept and not refused (#429).
#[test]
fn compile_reports_the_sweep_or_why_it_was_not_swept() {
    let machine = self_machine();
    let audit = machine.audit_with(SWEEP_BUDGET, is_engine_owned).unwrap();
    assert_eq!(audit.valuations, 47);
    assert_eq!(sweep_report(&machine, SWEEP_BUDGET), audit.to_string());
    let over = AuditError::Budget {
        phase: "verify".into(),
        result: "pass".into(),
        valuations: 47,
        budget: 46,
    };
    assert_eq!(sweep_report(&machine, 46), format!("{over}\n"));
}
